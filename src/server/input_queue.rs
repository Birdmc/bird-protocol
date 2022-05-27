use log::debug;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::{OwnedReadHalf};
use crate::bytes::{InputByteQueue, InputByteQueueError, InputByteQueueResult};
use crate::protocol::{Readable, ReadError, VarInt};

pub struct ProtocolServerInputQueue<const BUFFER_SIZE: usize> {
    read: OwnedReadHalf,
    offset: usize,
    length: usize,
    current_buffer_size: usize,
    current_buffer_offset: usize,
    buffer: [u8; BUFFER_SIZE],
}

impl<const BUFFER_SIZE: usize> ProtocolServerInputQueue<BUFFER_SIZE> {
    pub fn new(read: OwnedReadHalf) -> ProtocolServerInputQueue<BUFFER_SIZE> {
        ProtocolServerInputQueue {
            read,
            buffer: [0; BUFFER_SIZE],
            offset: 0,
            length: 0,
            current_buffer_offset: 0,
            current_buffer_size: 0,
        }
    }

    pub async fn update(&mut self) -> Result<(), ReadError> {
        self.offset = 0;
        self.length = 5; // max size of VarInt
        self.length = <VarInt as Readable>::read(self).await?.0 as usize;
        self.offset = 0;
        Ok(())
    }

    pub async fn read_next(&mut self) -> InputByteQueueResult<()> {
        self.current_buffer_offset = 0;
        self.current_buffer_size = match self.read
            .read(&mut self.buffer)
            .await
            .map_err(|err| InputByteQueueError::Custom(err.to_string()))? {
            0 => return Err(InputByteQueueError::Custom("TcpStream closed while reading".into())),
            n => n,
        };
        debug!("Received chunk of data with size {}: {:?}",
            self.current_buffer_size, &self.buffer[0..self.current_buffer_size]);
        Ok(())
    }
}

macro_rules! take_slice {
    ($self: expr, $length: expr, $push_f: expr) => {
        match $self.has_bytes($length) {
            false => Err(InputByteQueueError::NoBytesLeft($self.length, $self.length)),
            true => {
                let mut current: usize = 0;
                loop {
                    if $self.current_buffer_offset == $self.current_buffer_size {
                        $self.read_next().await?
                    }
                    $self.offset += 1;
                    $push_f(current);
                    current += 1;
                    $self.current_buffer_offset += 1;
                    if current == $length {
                        break;
                    }
                }
                Ok(())
            }
        }
    }
}

#[async_trait::async_trait]
impl<const BUFFER_SIZE: usize> InputByteQueue for ProtocolServerInputQueue<BUFFER_SIZE> {
    async fn take_byte(&mut self) -> InputByteQueueResult<u8> {
        match self.offset == self.length {
            true => Err(InputByteQueueError::NoBytesLeft(self.length, self.length)),
            false => {
                if self.current_buffer_offset == self.current_buffer_size {
                    self.read_next().await?
                }
                self.offset += 1;
                let byte = self.buffer[self.current_buffer_offset];
                self.current_buffer_offset += 1;
                Ok(byte)
            }
        }
    }


    async fn take_bytes(&mut self, into: &mut [u8]) -> InputByteQueueResult<()> {
        take_slice!(self, into.len(), |current| into[current] = self.buffer[self.current_buffer_offset])
    }

    async fn take_vec(&mut self, length: usize, into: &mut Vec<u8>) -> InputByteQueueResult<()> {
        take_slice!(self, length, |_| into.push(self.buffer[self.current_buffer_offset]))
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