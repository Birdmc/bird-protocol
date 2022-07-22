use anyhow::Error;

#[derive(Debug, thiserror::Error)]
pub enum PacketReadableError {
    #[error("Bytes exceeded")]
    BytesExceeded,
    #[error("{0}")]
    Any(#[from] anyhow::Error),
}

pub trait PacketReadable<'a>: Sized {
    fn read(read: &mut PacketRead<'a>) -> Result<Self, PacketReadableError>;
}

pub trait PacketVariantReadable<'a, T: Sized> {
    fn read_variant(read: &mut PacketRead<'a>) -> Result<T, PacketReadableError>;
}

pub trait PacketWritable {
    fn write<W>(&self, write: &mut W) -> Result<(), anyhow::Error> where W: PacketWrite;
}

pub trait PacketVariantWritable<T: ?Sized> {
    fn write_variant<W>(object: &T, write: &mut W) -> Result<(), anyhow::Error> where W: PacketWrite;
}

impl<'a, T: PacketReadable<'a>> PacketVariantReadable<'a, T> for T {
    fn read_variant(read: &mut PacketRead<'a>) -> Result<T, PacketReadableError> {
        T::read(read)
    }
}

impl<T: PacketWritable> PacketVariantWritable<T> for T {
    fn write_variant<W>(object: &T, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        object.write(write)
    }
}

pub trait PacketWrite {
    fn write_byte(&mut self, byte: u8) -> Result<(), anyhow::Error>;

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), anyhow::Error>;

    fn write_bytes_owned(&mut self, bytes: Vec<u8>) -> Result<(), anyhow::Error>;

    fn write_bytes_fixed<const SIZE: usize>(&mut self, bytes: [u8; SIZE]) -> Result<(), anyhow::Error>;
}

pub struct PacketRead<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> PacketRead<'a> {
    pub fn new(bytes: &'a [u8]) -> PacketRead {
        PacketRead { bytes, offset: 0 }
    }

    pub fn take_byte(&mut self) -> Result<u8, PacketReadableError> {
        match self.offset == self.bytes.len() {
            true => Err(PacketReadableError::BytesExceeded),
            false => {
                // Safety. offset is always less than bytes length and we already checked
                // that bytes length and offset is not equal. So offset is less and we can get by offset index
                let byte = *unsafe { self.bytes.get_unchecked(self.offset) };
                self.offset += 1;
                Ok(byte)
            }
        }
    }

    pub fn take_slice(&mut self, length: usize) -> Result<&'a [u8], PacketReadableError> {
        match self.is_available(length) {
            true => {
                let previous_offset = self.offset;
                self.offset += length;
                Ok(&self.bytes[previous_offset..self.offset])
            }
            false => Err(PacketReadableError::BytesExceeded)
        }
    }

    pub const fn available(&self) -> usize {
        // Panics. never offset is always less than length of bytes
        self.bytes.len() - self.offset
    }

    pub const fn is_available(&self, bytes: usize) -> bool {
        self.available() >= bytes
    }
}

#[derive(Debug, Default)]
pub struct VecPacketWrite {
    pub vec: Vec<u8>
}

impl PacketWrite for VecPacketWrite {
    fn write_byte(&mut self, byte: u8) -> Result<(), Error> {
        Ok(self.vec.push(byte))
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        Ok(self.vec.extend_from_slice(bytes))
    }

    fn write_bytes_owned(&mut self, bytes: Vec<u8>) -> Result<(), Error> {
        self.write_bytes(bytes.as_slice())
    }

    fn write_bytes_fixed<const SIZE: usize>(&mut self, bytes: [u8; SIZE]) -> Result<(), Error> {
        self.write_bytes(bytes.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn packet_read() {
        let mut packet_read = PacketRead::new(&[0, 2, 3]);
        assert_eq!(packet_read.available(), 3);
        assert_eq!(packet_read.is_available(3), true);
        assert_eq!(packet_read.is_available(0), true);
        assert_eq!(packet_read.is_available(4), false);
        assert_eq!(packet_read.take_byte().unwrap(), 0);
        assert_eq!(packet_read.take_slice(2).unwrap(), &[2, 3]);
        assert_eq!(packet_read.available(), 0);
        assert_eq!(packet_read.is_available(1), false);
        assert_eq!(match packet_read.take_byte().unwrap_err() {
            PacketReadableError::BytesExceeded => true,
            _ => false
        }, true);
    }
}