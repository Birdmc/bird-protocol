use crate::packet::{InputPacketBytes, InputPacketBytesError, InputPacketBytesResult, OutputPacketBytes, OutputPacketBytesResult};

#[derive(Debug, Default)]
pub struct OutputPacketBytesVec {
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct InputPacketBytesPrepared {
    pub data: Box<[u8]>,
    pub offset: usize,
}

impl OutputPacketBytesVec {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl OutputPacketBytes for OutputPacketBytesVec {
    async fn write_byte(&mut self, byte: u8) -> OutputPacketBytesResult {
        Ok(self.data.push(byte))
    }

    async fn write_bytes(&mut self, slice: &[u8]) -> OutputPacketBytesResult {
        let mut index = self.data.len();
        self.data.resize(self.data.len() + slice.len(), 0);
        Ok(
            slice.iter().for_each(|byte| {
                self.data[index] = *byte;
                index += 1;
            })
        )
    }
}

impl From<Vec<u8>> for OutputPacketBytesVec {
    fn from(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl From<OutputPacketBytesVec> for Vec<u8> {
    fn from(data: OutputPacketBytesVec) -> Self {
        data.data
    }
}

impl From<OutputPacketBytesVec> for InputPacketBytesPrepared {
    fn from(output: OutputPacketBytesVec) -> Self {
        InputPacketBytesPrepared {
            data: output.data.into_boxed_slice(),
            offset: 0,
        }
    }
}

#[async_trait::async_trait]
impl InputPacketBytes for InputPacketBytesPrepared {
    async fn take_byte(&mut self) -> InputPacketBytesResult<u8> {
        match self.has_bytes(1) {
            true => {
                let byte = self.data[self.offset];
                self.offset += 1;
                Ok(byte)
            },
            false => Err(InputPacketBytesError::NoBytes(self.data.len())),
        }
    }

    async fn take_slice(&mut self, slice: &mut [u8]) -> InputPacketBytesResult<()> {
        match self.has_bytes(slice.len()) {
            true => {
                for index in 0..slice.len() {
                    slice[index] = self.data[self.offset];
                    self.offset += 1;
                }
                Ok(())
            },
            false => Err(InputPacketBytesError::NoBytes(self.data.len())),
        }
    }

    async fn take_vec(&mut self, vec: &mut Vec<u8>, count: usize) -> InputPacketBytesResult<()> {
        match self.has_bytes(count) {
            true => {
                vec.resize(count, 0);
                for index in 0..count {
                    vec[index] = self.data[self.offset];
                    self.offset += 1;
                }
                Ok(())
            },
            false => Err(InputPacketBytesError::NoBytes(self.data.len()))
        }
    }

    fn has_bytes(&self, count: usize) -> bool {
        self.remaining_bytes() >= count
    }

    fn remaining_bytes(&self) -> usize {
        match self.data.len() <= self.offset {
            true => 0,
            false => self.data.len() - self.offset + 1,
        }
    }
}

impl From<Vec<u8>> for InputPacketBytesPrepared {
    fn from(data: Vec<u8>) -> Self {
        InputPacketBytesPrepared {
            data: data.into_boxed_slice(),
            offset: 0
        }
    }
}