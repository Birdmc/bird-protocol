use anyhow::Error;
use bytes::{BufMut, BytesMut};
use crate::packet::PacketWrite;

impl PacketWrite for BytesMut {
    fn write_byte(&mut self, byte: u8) -> Result<(), Error> {
        Ok(self.put_u8(byte))
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        Ok(self.put_slice(bytes))
    }

    fn write_bytes_owned(&mut self, bytes: Vec<u8>) -> Result<(), Error> {
        self.write_bytes(bytes.as_slice())
    }

    fn write_bytes_fixed<const SIZE: usize>(&mut self, bytes: [u8; SIZE]) -> Result<(), Error> {
        self.write_bytes(bytes.as_slice())
    }
}