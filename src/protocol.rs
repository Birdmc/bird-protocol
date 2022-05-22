use crate::bytes::{InputByteQueue, InputByteQueueError, OutputByteQueue};
use std::mem;
use std::str::{from_utf8, Utf8Error};
use cubic_chat::component::ComponentType;
use cubic_chat::identifier::{Identifier, IdentifierError};
use serde_json::Error;
use uuid::Uuid;

pub enum WriteError {
    JSON(serde_json::Error),
}

pub enum ReadError {
    BadVarNum,
    BadUtf8(Utf8Error),
    BadStringLimit(i32),
    BadIdentifier(IdentifierError),
    BadJson(serde_json::Error),
    InputQueue(InputByteQueueError),
}

impl From<serde_json::Error> for WriteError {
    fn from(val: Error) -> Self {
        WriteError::JSON(val)
    }
}

impl From<Utf8Error> for ReadError {
    fn from(err: Utf8Error) -> Self {
        ReadError::BadUtf8(err)
    }
}

impl From<IdentifierError> for ReadError {
    fn from(err: IdentifierError) -> Self {
        ReadError::BadIdentifier(err)
    }
}

impl From<serde_json::Error> for ReadError {
    fn from(err: Error) -> Self {
        ReadError::BadJson(err)
    }
}

impl From<InputByteQueueError> for ReadError {
    fn from(err: InputByteQueueError) -> Self {
        ReadError::InputQueue(err)
    }
}

macro_rules! delegate_type {
    ($name: ident, $delegates: ident) => {
        #[derive(Copy, Clone, Debug, Default)]
        pub struct $name($delegates);

        impl From<$delegates> for $name {
            fn from(val: $delegates) -> Self {
                $name(val)
            }
        }

        impl From<$name> for $delegates {
            fn from(val: $name) -> Self {
                val.0
            }
        }
    }
}

macro_rules! protocol_num_type {
    ($type: ident) => {
        impl Writable for $type {
            fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
                output.put_bytes(&self.to_le_bytes());
                Ok(())
            }
        }

        impl Readable for $type {
            fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
                let mut bytes = [0_u8; mem::size_of::<$type>()];
                input.take_bytes(&mut bytes)?;
                Ok($type::from_le_bytes(bytes))
            }
        }
    }
}

const VAR_NUM_SEGMENT_BITS: i32 = 0x7F;
const VAR_NUM_CONTINUE_BIT: i32 = 0x80;
const VAR_NUM_R_CONTINUE_BIT: i32 = !VAR_NUM_CONTINUE_BIT;

macro_rules! protocol_var_num_type {
    ($type: ident, $num_type: ident) => {
        impl Writable for $type {
            fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError>{
                let mut value = $num_type::from(*self);
                loop {
                    if ((value & (VAR_NUM_R_CONTINUE_BIT as $num_type)) == 0) {
                        output.put_byte(value as u8);
                        break;
                    }
                    output.put_byte(
                        ((value & (VAR_NUM_SEGMENT_BITS as $num_type)) | (VAR_NUM_CONTINUE_BIT as $num_type)) as u8
                    );
                    value >>= 7;
                }
                Ok(())
            }
        }

        impl Readable for $type {
            fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
                const BITS: $num_type = (mem::size_of::<$num_type>() * 8) as $num_type;
                let mut value: $num_type = 0;
                let mut position: $num_type = 0;
                loop {
                    let current_byte = input.take_byte()? as $num_type;
                    value |= (current_byte & (VAR_NUM_SEGMENT_BITS as $num_type)) << position;
                    if ((current_byte & (VAR_NUM_CONTINUE_BIT as $num_type)) == 0) {
                        break;
                    }
                    position += 7;
                    if (position >= BITS) {
                        return Err(ReadError::BadVarNum)
                    }
                }
                Ok($type::from(value))
            }
        }
    }
}

pub trait Writable {
    fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError>;
}

pub trait Readable: Sized {
    fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError>;
}

impl Writable for u8 {
    fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
        output.put_byte(*self);
        Ok(())
    }
}

impl Readable for u8 {
    fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
        Ok(input.take_byte()?)
    }
}

impl Writable for i8 {
    fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
        (*self as u8).write(output)
    }
}

impl Readable for i8 {
    fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
        u8::read(input).map(|val| val as i8)
    }
}

impl Writable for bool {
    fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
        match self {
            true => 1_u8,
            false => 0_u8,
        }.write(output)
    }
}

impl Readable for bool {
    fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
        u8::read(input).map(|val| val != 0)
    }
}

protocol_num_type!(i16);
protocol_num_type!(u16);
protocol_num_type!(i32);
protocol_num_type!(u32);
protocol_num_type!(i64);
protocol_num_type!(u64);
protocol_num_type!(i128);
protocol_num_type!(u128);
protocol_num_type!(f32);
protocol_num_type!(f64);

delegate_type!(VarInt, i32);
delegate_type!(VarLong, i64);
protocol_var_num_type!(VarInt, i32);
protocol_var_num_type!(VarLong, i64);

impl Readable for Uuid {
    fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
        let mut bytes = [0_u8; 16];
        input.take_bytes(&mut bytes)?;
        Ok(Uuid::from_bytes(bytes))
    }
}

impl Writable for Uuid {
    fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
        output.put_bytes(self.as_bytes());
        Ok(())
    }
}

const STRING_LIMIT: i32 = 32767;
const CHAT_LIMIT: i32 = 262144;

fn read_string_with_limit(input: &mut impl InputByteQueue, limit: i32) -> Result<String, ReadError> {
    let length: i32 = VarInt::read(input)?.into();
    match length > limit {
        true => Err(ReadError::BadStringLimit(limit)),
        false => {
            let slice = input.take_slice(length as usize)?;
            Ok(from_utf8(slice)?.into())
        }
    }
}

impl Readable for String {
    fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
        read_string_with_limit(input, STRING_LIMIT)
    }
}

impl Writable for String {
    fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
        output.put_bytes(self.as_bytes());
        Ok(())
    }
}

impl Readable for Identifier {
    fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
        let str = String::read(input)?;
        Ok(Identifier::from_full(str)?)
    }
}

impl Writable for Identifier {
    fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
        self.to_string().write(output)
    }
}

impl Readable for ComponentType {
    fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
        let str = read_string_with_limit(input, CHAT_LIMIT)?;
        Ok(serde_json::from_str(&*str)?)
    }
}

impl Writable for ComponentType {
    fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
        let str = serde_json::to_string(self)?;
        str.write(output)
    }
}