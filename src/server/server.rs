use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;
use log::{debug, error, warn};
use tokio::io::{AsyncWriteExt};
use tokio::join;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::mpsc::error::SendError;
use crate::protocol::{VarInt, Writable, WriteError};
use crate::server::input_queue::ProtocolServerInputQueue;
use crate::tokio::BytesOutputQueue;
use crate::version::{PacketNode, State};

const BUFFER_SIZE: usize = 2048;

#[derive(Debug)]
pub enum WriteObjectError {
    Send(SendError<ConnectionMessage>),
    Write(WriteError),
}

#[derive(Debug)]
pub enum ConnectionMessage {
    Bytes(Vec<u8>),
    Close,
}

pub struct Connection {
    addr: SocketAddr,
    pub(crate) sender: Sender<ConnectionMessage>,
}

pub trait ProtocolServerHandler<R: PacketNode>: Send + Sync {
    fn handle_connect(&self, connection: Arc<Connection>);

    fn handle_disconnect(&self, connection: Arc<Connection>);

    fn handle_event(&self, connection: Arc<Connection>, state: &mut State, packet: R);
}

pub struct ProtocolServer<H: ProtocolServerHandler<R>, R: PacketNode + Send + Sync> {
    pub handler: H,
    pub host: String,
    pub packet_node: PhantomData<R>,
}

impl Connection {
    pub(crate) fn new(addr: SocketAddr) -> (Connection, Receiver<ConnectionMessage>) {
        let (sender, receiver) = channel(2048);
        (Connection { sender, addr }, receiver)
    }

    pub fn get_addr(&self) -> &SocketAddr {
        &self.addr
    }

    pub fn set_addr(&mut self, addr: SocketAddr) {
        self.addr = addr;
    }

    pub async fn write_bytes(&self, bytes: Vec<u8>) -> Result<(), SendError<ConnectionMessage>> {
        self.sender.send(ConnectionMessage::Bytes(bytes)).await
    }

    pub async fn write_object(&self, writable: &impl Writable) -> Result<(), WriteObjectError> {
        self.sender.send(ConnectionMessage::Bytes(
            self
                .prepare_object(writable)
                .map_err(|err| WriteObjectError::Write(err))?
        ))
            .await
            .map_err(|err| WriteObjectError::Send(err))
    }

    pub fn prepare_object(&self, writable: &impl Writable) -> Result<Vec<u8>, WriteError> {
        let mut output = BytesOutputQueue::new();
        writable.write(&mut output)?;
        let mut bytes = output.get_bytes_vec();
        Connection::prefix_length(&mut bytes)?;
        Ok(bytes)
    }

    pub fn prefix_length(vec: &mut Vec<u8>) -> Result<(), WriteError> {
        let mut length_bytes = BytesOutputQueue::new();
        VarInt(vec.len() as i32).write(&mut length_bytes)?;
        for byte in length_bytes.get_bytes_vec().iter().rev() {
            vec.insert(0, *byte);
        }
        Ok(())
    }

    pub async fn close(&self) -> Result<(), SendError<ConnectionMessage>> {
        self.sender.send(ConnectionMessage::Close).await
    }
}

impl<H: ProtocolServerHandler<R> + 'static, R: PacketNode + Send + Sync + 'static> ProtocolServer<H, R> {
    pub async fn run(self) -> tokio::io::Result<()> {
        let server = Arc::new(self);
        let listener = TcpListener::bind(&server.host).await?;
        loop {
            let (stream, addr) = match listener.accept().await {
                Ok((stream, addr)) => {
                    debug!("Accepted: {}", addr);
                    (stream, addr)
                }
                Err(e) => {
                    error!("Failed to accept client: {}", e);
                    continue;
                }
            };
            let server = server.clone();
            tokio::spawn(async move {
                let (connection, receiver) = Connection::new(addr);
                let connection = Arc::new(connection);
                let (read_half, write_half) = stream.into_split();
                let (c1, s1) = (connection.clone(), server.clone());
                let (c2, s2) = (connection.clone(), server.clone());
                join!(
                    ProtocolServer::read(s1, read_half, c1),
                    ProtocolServer::write(s2, write_half, c2, receiver)
                );
            });
        }
    }

    async fn read(server: Arc<ProtocolServer<H, R>>, read_half: OwnedReadHalf, connection: Arc<Connection>) {
        let mut state = State::Handshake;
        server.handler.handle_connect(connection.clone());
        let mut input_queue = ProtocolServerInputQueue::<BUFFER_SIZE>::new(read_half);
        loop {
            if let Err(err) = input_queue.update().await {
                warn!("Failed to read bytes ({}:{}): {:?}", file!(), line!(), err);
                connection.close().await.unwrap();
                break;
            }
            match R::read(state, &mut input_queue).await {
                Ok(packet) => {
                    server.handler.handle_event(connection.clone(), &mut state, packet);
                }
                Err(err) => {
                    warn!("Failed to read bytes ({}:{}): {:?}", file!(), line!(), err);
                    connection.close().await.unwrap();
                    break;
                }
            };
        }
    }

    async fn write(server: Arc<ProtocolServer<H, R>>, mut write_half: OwnedWriteHalf,
                   connection: Arc<Connection>, mut receiver: Receiver<ConnectionMessage>) {
        while let Some(message) = receiver.recv().await {
            debug!("Received message from write receiver: {:?}", message);
            match message {
                ConnectionMessage::Bytes(bytes) =>
                    if let Err(err) = write_half.write_all(bytes.as_slice()).await {
                        warn!("Failed to write bytes: {:?}", err);
                    },
                ConnectionMessage::Close => {
                    write_half.shutdown().await.unwrap();
                    server.handler.handle_disconnect(connection);
                    break;
                }
            }
        }
    }
}