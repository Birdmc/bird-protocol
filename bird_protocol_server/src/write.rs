use std::sync::Arc;
use tokio::*;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::mpsc::Receiver;
use tokio::task::yield_now;
use crate::connection::Connection;
use crate::handler::{ConnectionHandler, ReadHandler};
use crate::server::ProtocolServerDeclare;

pub enum WriteMessage {
    Close,
    Bytes(Vec<u8>),
}

pub(crate) struct WriteStreamQueue {
    pub write_half: OwnedWriteHalf,
    pub receiver: Receiver<WriteMessage>,
}

impl WriteStreamQueue {
    pub async fn run<
        H: ReadHandler + Sized + Send + Sync + 'static,
        C: ConnectionHandler + Sized + Send + Sync + 'static
    >(mut self, connection: Arc<Connection>, declare: Arc<ProtocolServerDeclare<H, C>>) -> io::Result<()> {
        while let Some(bytes) = self.receiver.recv().await {
            match bytes {
                WriteMessage::Close => {
                    declare.connection_handler.handle_disconnect(connection);
                    return self.write_half.shutdown().await;
                }
                WriteMessage::Bytes(bytes) =>
                    self.write_half.write_all(bytes.as_slice()).await?,
            }
            yield_now().await;
        }
        Ok(())
    }
}