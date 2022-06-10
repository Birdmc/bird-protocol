use cubic_protocol::packet::{CustomError, InputPacketBytes, InputPacketBytesError, InputPacketBytesResult, PacketReadableResult};
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;

pub(crate) struct ReadQueue<const BUFFER_SIZE: usize> {
    read_half: OwnedReadHalf,
    packet_length: usize,
    packet_offset: usize,
    buffer: [u8; BUFFER_SIZE],
    buffer_size: usize,
    buffer_offset: usize,
}

impl<const BUFFER_SIZE: usize> ReadQueue<BUFFER_SIZE> {
    pub fn new(read_half: OwnedReadHalf) -> Self {
        Self {
            read_half,
            packet_length: 0,
            packet_offset: 0,
            buffer: [0; BUFFER_SIZE],
            buffer_size: 0,
            buffer_offset: 0,
        }
    }

    async fn read_next_bytes(&mut self) -> InputPacketBytesResult<()> {
        match self.read_half.read(&mut self.buffer).await {
            Ok(len) => {
                self.buffer_size = len;
                self.buffer_offset = 0;
                Ok(())
            }
            Err(err) => Err(
                CustomError::StaticStr("Connection was closed during reading").into()
            )
        }
    }

    async fn read_next_bytes_if_need(&mut self) -> InputPacketBytesResult<()> {
        match self.buffer_offset == self.buffer_size {
            true => self.read_next_bytes().await,
            false => Ok(())
        }
    }

    pub async fn next_packet(&mut self) -> PacketReadableResult<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl<const BUFFER_SIZE: usize> InputPacketBytes for ReadQueue<BUFFER_SIZE> {
    async fn take_byte(&mut self) -> InputPacketBytesResult<u8> {
        match self.packet_offset == self.packet_length {
            true => Err(InputPacketBytesError::NoBytes(self.packet_length)),
            false => {
                self.read_next_bytes_if_need().await?;
                let byte = self.buffer[self.buffer_offset];
                self.buffer_offset += 1;
                self.packet_offset += 1;
                Ok(byte)
            }
        }
    }

    async fn take_slice(&mut self, slice: &mut [u8]) -> InputPacketBytesResult<()> {
        match self.has_bytes(slice.len()) {
            true => {
                Ok(())
            },
            false => Err(InputPacketBytesError::NoBytes(self.packet_length)),
        }
    }

    async fn take_vec(&mut self, vec: &mut Vec<u8>) -> InputPacketBytesResult<()> {
        Ok(())
    }

    fn has_bytes(&self, count: usize) -> bool {
        self.remaining_bytes() >= count
    }

    fn remaining_bytes(&self) -> usize {
        match self.packet_length >= self.packet_offset {
            true => 0,
            false => self.packet_length - self.packet_offset + 1,
        }
    }
}