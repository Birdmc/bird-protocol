use crate::packet::{InputPacketBytes, InputPacketBytesError, InputPacketBytesResult, OutputPacketBytes, OutputPacketBytesResult};

#[derive(Debug)]
pub struct OutputPacketBytesVec {
    pub data: Vec<u8>,
}

#[derive(Debug)]
struct InputPacketBytesPrepared {
    pub data: Box<[u8]>,
    pub offset: usize,
}

#[async_trait::async_trait]
impl OutputPacketBytes for OutputPacketBytesVec {
    async fn write_byte(&mut self, byte: u8) -> OutputPacketBytesResult {
        Ok(self.data.push(byte))
    }

    async fn write_bytes(&mut self, slice: &[u8]) -> OutputPacketBytesResult {
        self.data.resize(self.0.len() + slice.len(), 0);
        let mut index = self.0.len();
        Ok(
            slice.iter().for_each(|byte| {
                self.0[index] = *byte;
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
                for index in slice.len() {
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
                for index in count {
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