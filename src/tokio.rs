use bytes::{BufMut, BytesMut};
use crate::bytes::{InputByteQueue, InputByteQueueError, InputByteQueueResult, OutputByteQueue};

pub struct BytesInputQueue {
    offset: usize,
    length: usize,
    bytes: BytesMut,
}

pub struct BytesOutputQueue {
    bytes: BytesMut,
}

impl BytesInputQueue {
    pub fn new(length: usize, bytes: BytesMut) -> BytesInputQueue {
        BytesInputQueue {
            offset: 0,
            length, bytes
        }
    }

    pub fn new_without_slice(bytes: BytesMut) -> BytesInputQueue {
        BytesInputQueue::new(bytes.len(), bytes)
    }
}

#[async_trait::async_trait]
impl InputByteQueue for BytesInputQueue {
    async fn take_byte(&mut self) -> InputByteQueueResult<u8> {
        match self.length == self.offset {
            true => Err(InputByteQueueError::NoBytesLeft(self.offset, self.length)),
            false => {
                let byte = self.bytes[self.offset];
                self.offset += 1;
                Ok(byte)
            }
        }
    }

    async fn take_bytes(&mut self, into: &mut [u8]) -> InputByteQueueResult<()> {
        match self.has_bytes(into.len()) {
            false => Err(InputByteQueueError::NoBytesLeft(self.length, self.length)),
            true => {
                for i in 0..into.len() {
                    into[i] = self.bytes[self.offset];
                    self.offset += 1;
                }
                Ok(())
            }
        }
    }

    async fn take_vec(&mut self, length: usize, into: &mut Vec<u8>) -> InputByteQueueResult<()> {
        match self.has_bytes(length) {
            false => Err(InputByteQueueError::NoBytesLeft(self.length, self.length)),
            true => {
                for _ in 0..length {
                    into.push(self.bytes[self.offset]);
                    self.offset += 1;
                }
                Ok(())
            }
        }
    }

    fn has_bytes(&mut self, bytes: usize) -> bool {
        self.remaining_bytes() >= bytes
    }

    fn remaining_bytes(&self) -> usize {
        match self.length > self.offset {
            true => self.length - self.offset,
            false => 0
        }
    }
}

impl BytesOutputQueue {
    pub fn new() -> BytesOutputQueue {
        BytesOutputQueue {
            bytes: BytesMut::new(),
        }
    }

    pub fn get_bytes(self) -> BytesMut {
        self.bytes
    }

    pub fn get_bytes_vec(self) -> Vec<u8> {
        self.bytes.to_vec()
    }
}

impl OutputByteQueue for BytesOutputQueue {
    fn put_byte(&mut self, byte: u8) {
        self.bytes.put_u8(byte)
    }

    fn put_bytes(&mut self, bytes: &[u8]) {
        self.bytes.put_slice(bytes)
    }
}