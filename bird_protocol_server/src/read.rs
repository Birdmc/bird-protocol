use bird_protocol::packet::{CustomError, InputPacketBytes, InputPacketBytesError, InputPacketBytesResult, PacketReadable, PacketReadableResult};
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;
use bird_protocol::types::VarInt;

#[derive(Clone, Copy)]
struct SlicePointer {
    ptr: *mut u8,
}

unsafe impl Send for SlicePointer {}

unsafe impl Sync for SlicePointer {}

pub struct ReadStreamQueue<const BUFFER_SIZE: usize> {
    read_half: OwnedReadHalf,
    packet_length: usize,
    packet_offset: usize,
    buffer: [u8; BUFFER_SIZE],
    buffer_size: usize,
    buffer_offset: usize,
}

impl<const BUFFER_SIZE: usize> From<OwnedReadHalf> for ReadStreamQueue<BUFFER_SIZE> {
    fn from(read_half: OwnedReadHalf) -> Self {
        ReadStreamQueue::new(read_half)
    }
}

impl<const BUFFER_SIZE: usize> From<ReadStreamQueue<BUFFER_SIZE>> for OwnedReadHalf {
    fn from(queue: ReadStreamQueue<BUFFER_SIZE>) -> Self {
        queue.read_half
    }
}

impl<const BUFFER_SIZE: usize> ReadStreamQueue<BUFFER_SIZE> {
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

    pub fn close(self) -> (OwnedReadHalf, Box<[u8]>) {
        (
            self.read_half,
            self.buffer[self.buffer_offset..self.buffer_size].into(),
        )
    }

    async fn read_next_bytes(&mut self) -> InputPacketBytesResult<()> {
        match self.read_half.read(&mut self.buffer).await {
            Ok(0) | Err(_) => Err(
                CustomError::StaticStr("Connection was closed during reading").into()
            ),
            Ok(len) => {
                self.buffer_size = len;
                self.buffer_offset = 0;
                log::debug!("Received bytes: {:?}", &self.buffer[0..self.buffer_size]);
                Ok(())
            }
        }
    }

    async fn read_next_bytes_if_need(&mut self) -> InputPacketBytesResult<()> {
        match self.buffer_offset == self.buffer_size {
            true => self.read_next_bytes().await,
            false => Ok(())
        }
    }

    pub async fn next_packet(&mut self) -> PacketReadableResult<()> {
        self.packet_length = 5; // maximum VarInt length
        self.packet_offset = 0;
        self.packet_length = <VarInt as PacketReadable>::read(self).await?.0 as usize;
        self.packet_offset = 0;
        Ok(())
    }

    async unsafe fn copy_into(&mut self, mut dst: SlicePointer, count: usize) -> InputPacketBytesResult<()> {
        let mut offset: usize = 0;
        loop {
            let can_copy = self.buffer_size - self.buffer_offset;
            let need_copy = count - offset;
            match need_copy > can_copy {
                true => {
                    std::ptr::copy_nonoverlapping(
                        self.buffer.as_ptr().add(self.buffer_offset), dst.ptr, can_copy,
                    );
                    dst.ptr = dst.ptr.add(can_copy);
                    offset += can_copy;
                    self.read_next_bytes().await?
                }
                false => {
                    std::ptr::copy_nonoverlapping(
                        self.buffer.as_ptr().add(self.buffer_offset), dst.ptr, need_copy,
                    );
                    self.buffer_offset += need_copy;
                    break Ok(());
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl<const BUFFER_SIZE: usize> InputPacketBytes for ReadStreamQueue<BUFFER_SIZE> {
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
            true => unsafe {
                let slice_pointer = SlicePointer { ptr: slice.as_mut_ptr() };
                self.copy_into(slice_pointer, slice.len()).await
            },
            false => Err(InputPacketBytesError::NoBytes(self.packet_length)),
        }
    }

    async fn take_vec(&mut self, vec: &mut Vec<u8>, count: usize) -> InputPacketBytesResult<()> {
        match self.has_bytes(count) {
            true => unsafe {
                vec.resize(count, 0);
                let slice_pointer = SlicePointer { ptr: vec.as_mut_ptr() };
                self.copy_into(slice_pointer, count).await
            },
            false => Err(InputPacketBytesError::NoBytes(self.packet_length))
        }
    }

    fn has_bytes(&self, count: usize) -> bool {
        self.remaining_bytes() >= count
    }

    fn remaining_bytes(&self) -> usize {
        match self.packet_length > self.packet_offset {
            true => self.packet_length - self.packet_offset,
            false => 0
        }
    }
}