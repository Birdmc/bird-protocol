use serde::Serialize;
use crate::packet::{CustomError, OutputPacketBytes, PacketWritable, PacketWritableResult};
use crate::types::{ReadProtocolNbt, WriteRemainingBytesArray};

#[async_trait::async_trait]
impl<T: Serialize + Sync + Send> PacketWritable for ReadProtocolNbt<T> {
    async fn write(&self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        WriteRemainingBytesArray::from(
            &fastnbt::to_bytes(&self.value)
                .map_err(|err| CustomError::Error(Box::new(err)))?
        )
            .write(output).await
    }
}