use crate::bytes::{InputByteQueue, InputByteQueueResult, OutputByteQueue, InputByteQueueError};
use std::mem;
use std::str::from_utf8;

pub struct InputProtocol<T: InputByteQueue> {
    queue: T,
}

pub struct OutputProtocol<T: OutputByteQueue> {
    queue: T,
}

macro_rules! read_num_method {
    ($method: ident, $type: ident) => {
        pub fn $method(&mut self) -> InputByteQueueResult<$type> {
            let mut bytes = [0_u8; mem::size_of::<$type>()];
            self.queue.take_bytes(&mut bytes)
                .map(|_| $type::from_le_bytes(bytes))
        }
    };
}

const SEGMENT_BITS: i32 = 0x7F;
const CONTINUE_BIT: i32 = 0x80;
const COMPLEMENT_SEGMENT_BITS: i32 = -0x80;

const STRING_LIMIT: usize = 32767;

macro_rules! read_var_num_method {
    ($method: ident, $type: ident) => {
        pub fn $method(&mut self) -> InputByteQueueResult<$type> {
            const BITS: usize = mem::size_of::<$type>() * 8;
            let mut value: $type = 0;
            let mut position: usize = 0;
            let mut current_byte: u8 = 0;
            loop {
                let current_byte = self.read_byte()? as $type;
                value |= (current_byte & (SEGMENT_BITS as $type)) << position;
                if ((current_byte & (CONTINUE_BIT as $type)) == 0) {
                    break;
                }
                position += 7;
                if (position >= BITS) {
                    return Err(InputByteQueueError::Custom("Bad var number".into()))
                }
            }
            Ok(value)
        }
    }
}

impl<T: InputByteQueue> InputProtocol<T> {
    pub fn new(queue: T) -> InputProtocol<T> {
        InputProtocol { queue }
    }

    pub fn read_byte(&mut self) -> InputByteQueueResult<i8> {
        self.read_unsigned_byte().map(|val| val as i8)
    }

    pub fn read_unsigned_byte(&mut self) -> InputByteQueueResult<u8> {
        self.queue.take_byte()
    }

    pub fn read_boolean(&mut self) -> InputByteQueueResult<bool> {
        self.read_unsigned_byte().map(|val| val != 0)
    }

    read_num_method!(read_short, i16);
    read_num_method!(read_unsigned_short, u16);
    read_num_method!(read_integer, i32);
    read_num_method!(read_unsigned_integer, u32);
    read_num_method!(read_long, i64);
    read_num_method!(read_unsigned_long, u64);
    read_num_method!(read_float, f32);
    read_num_method!(read_double, f64);
    read_var_num_method!(read_var_integer, i32);
    read_var_num_method!(read_var_long, i64);

    pub fn read_string_with_limit(&mut self, limit: usize) -> InputByteQueueResult<String> {
        let size = self.read_var_integer()? as usize;
        match size >= limit {
            true => Err(InputByteQueueError::Custom(format!("Too big string, max size: {}", limit))),
            false => {
                let slice = self.queue.take_slice(size)?;
                let string = from_utf8(slice)
                    .map_err(|err| InputByteQueueError::Custom(err.to_string()))?;
                Ok(string.to_string())
            }
        }
    }

    pub fn read_string(&mut self) -> InputByteQueueResult<String> {
        self.read_string_with_limit(STRING_LIMIT)
    }



}