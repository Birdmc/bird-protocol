use tokio::*;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc::Receiver;
use crate::server::ProtocolServerRuntime;

pub enum WriteMessage {
    Close,
    Bytes(Vec<u8>),
}

pub(crate) struct WriteQueue {
    pub write_half: OwnedWriteHalf,
    pub receiver: Receiver<WriteMessage>,
}

impl WriteQueue {
    pub async fn run(mut self) -> io::Result<()> {
        while let Some(bytes) = self.receiver.recv().await {
            match bytes {
                WriteMessage::Close =>
                    return self.write_half.shutdown().await,
                WriteMessage::Bytes(bytes) =>
                    self.write_half.write_all(bytes.as_slice()).await?,
            }
        }
        Ok(())
    }
}