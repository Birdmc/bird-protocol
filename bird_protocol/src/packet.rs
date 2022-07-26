use anyhow::Error;

#[derive(Debug, thiserror::Error)]
pub enum PacketReadableError {
    #[error("Bytes exceeded")]
    BytesExceeded,
    #[error("{0}")]
    Any(#[from] anyhow::Error),
}

pub trait PacketReadable<'a>: Sized {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a>;
}

pub trait PacketVariantReadable<'a, T: Sized> {
    fn read_variant<R>(read: &mut R) -> Result<T, PacketReadableError> where R: PacketRead<'a>;
}

pub trait PacketWritable {
    fn write<W>(&self, write: &mut W) -> Result<(), anyhow::Error> where W: PacketWrite;
}

pub trait PacketVariantWritable<T: ?Sized> {
    fn write_variant<W>(object: &T, write: &mut W) -> Result<(), anyhow::Error> where W: PacketWrite;
}

impl<'a, T: PacketReadable<'a>> PacketVariantReadable<'a, T> for T {
    fn read_variant<R>(read: &mut R) -> Result<T, PacketReadableError> where R: PacketRead<'a> {
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

pub trait PacketRead<'a> {
    fn take_byte(&mut self) -> Result<u8, PacketReadableError>;

    fn take_slice(&mut self, length: usize) -> Result<&'a [u8], PacketReadableError>;

    fn rollback(&mut self, length: usize) -> Result<(), anyhow::Error>;

    fn available(&self) -> usize;

    fn is_available(&self, bytes: usize) -> bool;
}

pub struct SlicePacketRead<'a> {
    pub bytes: &'a [u8],
    offset: usize,
}

impl<'a> SlicePacketRead<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        SlicePacketRead { bytes, offset: 0 }
    }
}

impl<'a> PacketRead<'a> for SlicePacketRead<'a> {
    fn take_byte(&mut self) -> Result<u8, PacketReadableError> {
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

    fn take_slice(&mut self, length: usize) -> Result<&'a [u8], PacketReadableError> {
        match self.is_available(length) {
            true => {
                let previous_offset = self.offset;
                self.offset += length;
                Ok(&self.bytes[previous_offset..self.offset])
            }
            false => Err(PacketReadableError::BytesExceeded)
        }
    }

    fn rollback(&mut self, length: usize) -> Result<(), Error> {
        match self.offset < length {
            true => Err(Error::msg("Can not rollback")),
            false => {
                self.offset -= length;
                Ok(())
            }
        }
    }

    fn available(&self) -> usize {
        // Panics. never offset is always less than length of bytes
        self.bytes.len() - self.offset
    }

    fn is_available(&self, bytes: usize) -> bool {
        self.available() >= bytes
    }
}

impl PacketWrite for Vec<u8> {
    fn write_byte(&mut self, byte: u8) -> Result<(), Error> {
        Ok(self.push(byte))
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        Ok(self.extend_from_slice(bytes))
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
        let mut packet_read = SlicePacketRead::new(&[0, 2, 3]);
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