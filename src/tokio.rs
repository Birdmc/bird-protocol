use bytes::BytesMut;
use crate::bytes::{InputByteQueue, InputByteQueueError, InputByteQueueResult};

pub struct BytesInputQueue {
    offset: usize,
    length: usize,
    bytes: BytesMut,
}

impl InputByteQueue for BytesInputQueue {
    fn take_byte(&mut self) -> InputByteQueueResult<u8> {
        match self.length == self.offset {
            true => Err(InputByteQueueError::NoBytesLeft(self.offset, self.length)),
            false => {
                let byte = self.bytes[self.offset];
                self.offset += 1;
                Ok(byte)
            }
        }
    }

    fn take_bytes(&mut self, into: &mut [u8]) -> InputByteQueueResult<()> {
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

    fn take_slice(&mut self, size: usize) -> InputByteQueueResult<&[u8]> {
        match self.has_bytes(size) {
            false => Err(InputByteQueueError::NoBytesLeft(self.length, self.length)),
            true => {
                let start = self.offset;
                self.offset += size;
                Ok(&self.bytes[start..self.offset])
            }
        }
    }

    fn has_bytes(&mut self, bytes: usize) -> bool {
        self.remaining_bytes() >= bytes
    }

    fn remaining_bytes(&self) -> usize {
        self.length - self.offset
    }
}