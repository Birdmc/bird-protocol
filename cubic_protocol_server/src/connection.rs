use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;
use cubic_protocol::packet::{CustomError, PacketWritable, PacketWritableResult};
use cubic_protocol::types::VarInt;
use crate::write::{WriteBytes, WriteMessage};

pub struct Connection {
    addr: SocketAddr,
    sender: Sender<WriteMessage>,
}

impl Connection {
    pub(crate) fn new(addr: SocketAddr, sender: Sender<WriteMessage>) -> Self {
        Self { addr, sender }
    }

    pub fn get_addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn set_addr(&mut self, addr: SocketAddr) {
        self.addr = addr;
    }

    pub async fn close(&self) -> Result<(), CustomError> {
        self.sender.send(WriteMessage::Close).await
            .map_err(|err| CustomError::String(err.to_string()))
    }

    pub async fn write_raw_bytes(&self, bytes: Vec<u8>) -> Result<(), CustomError>{
        self.sender.send(WriteMessage::Bytes(bytes)).await
            .map_err(|err| CustomError::String(err.to_string()))
    }

    pub async fn write_bytes(&self, mut bytes: Vec<u8>) -> PacketWritableResult {
        let mut length_bytes = WriteBytes::default();
        VarInt::from(bytes.len() as i32).write(&mut length_bytes).await?;
        length_bytes.bytes.into_iter()
            .rev()
            .for_each(|byte| bytes.insert(0, byte));
        self.write_raw_bytes(bytes).await?;
        Ok(())
    }

    pub async fn write_object<T: PacketWritable>(&self, object: T) -> PacketWritableResult {
        let mut length_bytes = WriteBytes::default();
        object.write(&mut length_bytes).await?;
        self.write_bytes(length_bytes.bytes).await
    }
}