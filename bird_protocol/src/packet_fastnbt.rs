use std::marker::PhantomData;
use anyhow::Error;
use serde::Deserialize;
use crate::packet::{PacketRead, PacketReadable, PacketReadableError, PacketVariantReadable, PacketVariantWritable, PacketWrite};
use crate::packet_types::{ProtocolNbt, RemainingBytesSlice};

impl<'a, T: serde::Serialize> PacketVariantWritable<T> for ProtocolNbt {
    fn write_variant<W>(object: &T, write: &mut W) -> Result<(), anyhow::Error> where W: PacketWrite {
        RemainingBytesSlice::write_variant(&fastnbt::to_bytes(object)?, write)
    }
}

struct MemorizePacketRead<'b, 'a, R: PacketRead<'a>> {
    pub length: usize,
    pub input: &'b mut R,
    pub a_ph: PhantomData<&'a u8>,
}

impl<'b, 'a, R: PacketRead<'a>> MemorizePacketRead<'b, 'a, R> {
    fn skip_bytes(&mut self, size: usize) -> Result<(), anyhow::Error> {
        self.take_slice(size)?;
        self.length += size;
        Ok(())
    }
}

impl<'b, 'a, R: PacketRead<'a>> PacketRead<'a> for MemorizePacketRead<'b, 'a, R> {
    fn take_byte(&mut self) -> Result<u8, PacketReadableError> {
        let byte = self.input.take_byte()?;
        self.length += 1;
        Ok(byte)
    }

    fn take_slice(&mut self, length: usize) -> Result<&'a [u8], PacketReadableError> {
        let slice = self.input.take_slice(length)?;
        self.length += length;
        Ok(slice)
    }

    fn rollback(&mut self, length: usize) -> Result<(), Error> {
        self.length -= length;
        self.input.rollback(length)
    }

    fn available(&self) -> usize {
        self.input.available()
    }

    fn is_available(&self, bytes: usize) -> bool {
        self.input.is_available(bytes)
    }
}

fn read_nbt_length<'a, R>(input: &mut R) -> Result<usize, PacketReadableError> where R: PacketRead<'a> {
    let mut memorize = MemorizePacketRead { input, length: 0, a_ph: PhantomData };
    let tag = u8::read(&mut memorize)?;
    if tag != 0 {
        let length = u16::read(&mut memorize)?;
        memorize.skip_bytes(length as usize)?;
        skip_tag(tag, 1, &mut memorize)?
    }
    memorize.input.rollback(memorize.length)?;
    Ok(memorize.length)
}

fn skip_tag<'a, R: PacketRead<'a>>(
    tag: u8, count: usize, input: &mut MemorizePacketRead<'_, 'a, R>,
) -> Result<(), PacketReadableError> {
    Ok(match tag {
        0 => (),
        1 => input.skip_bytes(count)?,
        2 => input.skip_bytes(count * 2)?,
        3 | 5 => input.skip_bytes(count * 4)?,
        4 | 6 => input.skip_bytes(count * 8)?,
        7 => {
            let length = i32::read(input)?;
            if length <= 0 { return Ok(()); }
            input.skip_bytes(length as usize)?
        }
        8 => {
            let length = u16::read(input)?;
            input.skip_bytes(length as usize)?
        }
        9 => {
            let tag = u8::read(input)?;
            let size = i32::read(input)?;
            if size <= 0 { return Ok(()); }
            skip_tag(tag, size as usize, input)?
        }
        10 => loop {
            let tag = u8::read(input)?;
            if tag == 0 { break (); }
            let name_length = u16::read(input)?;
            input.skip_bytes(name_length as usize)?;
            skip_tag(tag, 1, input)?
        }
        11 => {
            let length = i32::read(input)?;
            if length <= 0 { return Ok(()); }
            input.skip_bytes(length as usize * 4)?
        }
        12 => {
            let length = i32::read(input)?;
            if length <= 0 { return Ok(()); }
            input.skip_bytes(length as usize * 8)?
        }
        _ => Err(anyhow::Error::msg("Bad nbt tag value"))?
    })
}

impl<'a, T: Deserialize<'a>> PacketVariantReadable<'a, T> for ProtocolNbt {
    fn read_variant<R>(read: &mut R) -> Result<T, PacketReadableError> where R: PacketRead<'a> {
        let length = read_nbt_length(read)?;
        fastnbt::from_bytes(read.take_slice(length)?)
            .map_err(|err| PacketReadableError::Any(err.into()))
    }
}