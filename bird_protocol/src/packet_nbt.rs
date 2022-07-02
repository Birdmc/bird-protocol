use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::packet::{CustomError, InputPacketBytes, InputPacketBytesResult, OutputPacketBytes, PacketReadable, PacketReadableResult, PacketWritable, PacketWritableResult};
use crate::types::{ReadProtocolNbt, WriteProtocolNbt, WriteRemainingBytesArray};

#[async_trait::async_trait]
impl<'a, T: Serialize + Sync + Send> PacketWritable for WriteProtocolNbt<'a, T> {
    async fn write(&self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        WriteRemainingBytesArray::from(
            &fastnbt::to_bytes(self.value)
                .map_err(|err| CustomError::Error(Box::new(err)))?
        )
            .write(output).await
    }
}

struct MemorizeInputPacketBytes<'a, T: InputPacketBytes> {
    pub bytes: Vec<u8>,
    pub input: &'a mut T,
}

impl<'a, T: InputPacketBytes> MemorizeInputPacketBytes<'a, T> {
    async fn skip_bytes(&mut self, size: usize) -> InputPacketBytesResult<()> {
        let mut vec = Vec::with_capacity(size);
        unsafe { vec.set_len(size); }
        self.take_vec(&mut vec, size).await
    }
}

#[async_trait::async_trait]
impl<'a, T: InputPacketBytes> InputPacketBytes for MemorizeInputPacketBytes<'a, T> {
    async fn take_byte(&mut self) -> InputPacketBytesResult<u8> {
        let byte = self.input.take_byte().await?;
        self.bytes.push(byte);
        Ok(byte)
    }

    async fn take_slice(&mut self, slice: &mut [u8]) -> InputPacketBytesResult<()> {
        self.input.take_slice(slice).await?;
        self.bytes.extend_from_slice(slice);
        Ok(())
    }

    async fn take_vec(&mut self, vec: &mut Vec<u8>, count: usize) -> InputPacketBytesResult<()> {
        self.input.take_vec(vec, count).await?;
        self.bytes.extend_from_slice(vec.as_slice());
        Ok(())
    }

    fn has_bytes(&self, count: usize) -> bool {
        self.input.has_bytes(count)
    }

    fn remaining_bytes(&self) -> usize {
        self.input.remaining_bytes()
    }
}

async fn read_nbt(input: &mut impl InputPacketBytes) -> PacketReadableResult<Vec<u8>> {
    let mut memorize = MemorizeInputPacketBytes { input, bytes: Vec::new() };
    let tag = u8::read(&mut memorize).await?;
    if tag != 0 {
        let length = u16::read(&mut memorize).await?;
        memorize.skip_bytes(length as usize).await?;
        skip_tag(tag, 1, &mut memorize).await?
    }
    Ok(memorize.bytes)
}

#[async_recursion::async_recursion]
async fn skip_tag<I: InputPacketBytes>(
    tag: u8, count: usize, input: &mut MemorizeInputPacketBytes<'_, I>,
) -> PacketReadableResult<()> {
    Ok(match tag {
        0 => (),
        1 => input.skip_bytes(count).await?,
        2 => input.skip_bytes(count * 2).await?,
        3 | 5 => input.skip_bytes(count * 4).await?,
        4 | 6 => input.skip_bytes(count * 8).await?,
        7 => {
            let length = i32::read(input).await?;
            if length <= 0 { return Ok(()); }
            input.skip_bytes(length as usize).await?
        }
        8 => {
            let length = u16::read(input).await?;
            input.skip_bytes(length as usize).await?
        }
        9 => {
            let tag = u8::read(input).await?;
            let size = i32::read(input).await?;
            if size <= 0 { return Ok(()); }
            skip_tag(tag, size as usize, input).await?
        }
        10 => loop {
            let tag = u8::read(input).await?;
            if tag == 0 { break (); }
            let name_length = u16::read(input).await?;
            input.skip_bytes(name_length as usize).await?;
            skip_tag(tag, 1, input).await?
        }
        11 => {
            let length = i32::read(input).await?;
            if length <= 0 { return Ok(()); }
            input.skip_bytes(length as usize * 4).await?
        }
        12 => {
            let length = i32::read(input).await?;
            if length <= 0 { return Ok(()); }
            input.skip_bytes(length as usize * 8).await?
        }
        _ => Err(CustomError::StaticStr("Bad nbt tag value"))?
    })
}

#[async_trait::async_trait]
impl<T: DeserializeOwned + Send + Sync> PacketReadable for ReadProtocolNbt<T> {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        let bytes = read_nbt(input).await?;
        Ok(ReadProtocolNbt {
            value: fastnbt::from_bytes(bytes.as_slice())
                .map_err(|err| CustomError::Error(Box::new(err)))?
        })
    }
}