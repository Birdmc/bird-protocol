use crate::bytes::{InputByteQueue, InputByteQueueError, OutputByteQueue};
use std::mem;
use std::str::{from_utf8, Utf8Error};
use cubic_chat::component::ComponentType;
use cubic_chat::identifier::{Identifier, IdentifierError};
use euclid::default::Vector3D;
use serde_json::Error;
use uuid::Uuid;

#[derive(Debug)]
pub enum WriteError {
    JSON(serde_json::Error),
}

#[derive(Debug)]
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
        #[derive(Copy, Clone, Debug, Default, PartialEq, PartialOrd, Hash)]
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

macro_rules! protocol_var_num_type {
    ($type: ident, $num_type: ident, $num_unsigned: ident) => {
        impl Writable for $type {
            fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError>{
                let mut value = $num_type::from(*self) as $num_unsigned;
                loop {
                    if ((value & 0x80) == 0) {
                        output.put_byte(value as u8);
                        break;
                    }
                    output.put_byte(((value as u8) & 0x7f) | 0x80);
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
                    value |= (current_byte & 0x7f) << position;
                    if ((current_byte & 0x80) == 0) {
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
protocol_var_num_type!(VarInt, i32, u32);
protocol_var_num_type!(VarLong, i64, u64);

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
        VarInt(self.len() as i32).write(output)?;
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

pub type BlockPosition = Vector3D<i32>;

const BLOCK_X_MASK: u64 = 0x3ffffff << 38;
const BLOCK_Z_MASK: u64 = 0x3ffffff << 12;
const BLOCK_Y_MASK: u64 = 0xfff;

const BLOCK_X_NEG_BOUND: i32 = 1 << 25;
const BLOCK_Z_NEG_BOUND: i32 = BLOCK_X_NEG_BOUND;
const BLOCK_Y_NEG_BOUND: i32 = 1 << 11;

impl Readable for BlockPosition {
    fn read(input: &mut impl InputByteQueue) -> Result<Self, ReadError> {
        let val = u64::read(input)?;
        let mut x = ((val & BLOCK_X_MASK) >> 38) as i32;
        let mut z = ((val & BLOCK_Z_MASK) >> 12) as i32;
        let mut y = (val & BLOCK_Y_MASK) as i32;
        if x >= BLOCK_X_NEG_BOUND {
            x -= BLOCK_X_NEG_BOUND << 1;
        }
        if z >= BLOCK_Z_NEG_BOUND {
            z -= BLOCK_Z_NEG_BOUND << 1;
        }
        if y >= BLOCK_Y_NEG_BOUND {
            y -= BLOCK_Y_NEG_BOUND << 1;
        }
        Ok(BlockPosition::new(x, y, z))
    }
}

impl Writable for BlockPosition {
    fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
        let x = self.x as i64;
        let z = self.z as i64;
        let y = self.y as i64;
        (((x & BLOCK_X_MASK as i64) << 38) |
            ((z & BLOCK_Z_MASK as i64) << 12) |
            (y & BLOCK_Y_MASK as i64)
        ).write(output)
    }
}

#[cfg(all(test, feature = "tokio-bytes"))]
mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};
    use crate::tokio::{BytesInputQueue, BytesOutputQueue};

    #[test]
    fn success_integer_test() {
        {
            let mut output = BytesOutputQueue::new();
            0xff321233_u32.write(&mut output).unwrap();
            let bytes = output.get_bytes();
            assert_eq!(bytes.len(), 4);
            assert_eq!(bytes[0], 0x33);
            assert_eq!(bytes[1], 0x12);
            assert_eq!(bytes[2], 0x32);
            assert_eq!(bytes[3], 0xff);
        }
        {
            let mut bytes = BytesMut::new();
            bytes.put_u8(0x97);
            bytes.put_u8(0x32);
            bytes.put_u8(0x11);
            bytes.put_u8(0xaa);
            let mut input = BytesInputQueue::new(4, bytes);
            assert_eq!(u32::read(&mut input).unwrap(), 0xaa113297_u32);
        }
        {
            const F: i64 = 33125;
            const S: i32 = 3294634;
            const T: u16 = 3219;
            let mut output = BytesOutputQueue::new();
            F.write(&mut output).unwrap();
            S.write(&mut output).unwrap();
            T.write(&mut output).unwrap();
            let bytes = output.get_bytes();
            let mut input = BytesInputQueue::new(bytes.len(), bytes);
            assert_eq!(i64::read(&mut input).unwrap(), F);
            assert_eq!(i32::read(&mut input).unwrap(), S);
            assert_eq!(u16::read(&mut input).unwrap(), T);
        }
    }

    #[test]
    fn success_var_num_test() {
        {
            let mut output = BytesOutputQueue::new();
            VarInt(0).write(&mut output).unwrap();
            let bytes = output.get_bytes();
            assert_eq!(bytes[0], 0);
        }
        {
            let mut output = BytesOutputQueue::new();
            VarInt(2097151).write(&mut output).unwrap();
            let bytes = output.get_bytes();
            assert_eq!(bytes.to_vec(), vec![255, 255, 127]);
        }
        {
            let mut output = BytesOutputQueue::new();
            VarInt(-1).write(&mut output).unwrap();
            let bytes = output.get_bytes();
            assert_eq!(bytes.to_vec(), vec![255, 255, 255, 255, 15]);
        }
        {
            let mut input = BytesInputQueue::new(
                1, BytesMut::from_iter(vec![0])
            );
            assert_eq!(VarInt::read(&mut input).unwrap(), VarInt(0));
        }
        {
            let mut input = BytesInputQueue::new(
                3, BytesMut::from_iter(vec![255, 255, 127])
            );
            assert_eq!(VarInt::read(&mut input).unwrap(), VarInt(2097151));
        }
        {
            let mut input = BytesInputQueue::new(
                5, BytesMut::from_iter(vec![255, 255, 255, 255, 15])
            );
            assert_eq!(VarInt::read(&mut input).unwrap(), VarInt(-1));
        }
    }

    #[test]
    fn success_string_test() {
        {
            const S: &str = "jenya705 is the best";
            let mut output = BytesOutputQueue::new();
            S.to_string().write(&mut output).unwrap();
            let mut input = BytesInputQueue::new_without_slice(output.get_bytes());
            assert_eq!(String::read(&mut input).unwrap(), S);
        }
    }
}