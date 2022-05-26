use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub enum InputByteQueueError {
    NoBytesLeft(usize, usize),
    Custom(String),
}

pub type InputByteQueueResult<T> = Result<T, InputByteQueueError>;

#[async_trait::async_trait]
pub trait InputByteQueue: Sync + Send {
    async fn take_byte(&mut self) -> InputByteQueueResult<u8>;

    async fn take_bytes(&mut self, into: &mut [u8]) -> InputByteQueueResult<()>;

    async fn take_slice(&mut self, size: usize) -> InputByteQueueResult<&[u8]>;

    fn has_bytes(&mut self, bytes: usize) -> bool;

    fn remaining_bytes(&self) -> usize;
}

pub trait OutputByteQueue {
    fn put_byte(&mut self, byte: u8);

    fn put_bytes(&mut self, bytes: &[u8]);
}

impl Display for InputByteQueueError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InputByteQueueError::NoBytesLeft(index, length) =>
                write!(f, "Count of the bytes is {}, there is not {} byte", length, index + 1),
            InputByteQueueError::Custom(str) =>
                write!(f, "Error: {}", str)
        }
    }
}

impl std::error::Error for InputByteQueueError {}