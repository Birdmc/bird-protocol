use std::marker::PhantomData;
use std::net::SocketAddr;
use std::sync::Arc;
use log::{error, warn};
use tokio::io::{AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::mpsc::error::SendError;
use cubic_protocol::protocol::{Writable, WriteError};
use cubic_protocol::tokio::BytesOutputQueue;
use cubic_protocol::version::{PacketNode, State};
use crate::input_queue::ProtocolServerInputQueue;

const BUFFER_SIZE: usize = 2048;

pub enum WriteObjectError {
    Send(SendError<ConnectionMessage>),
    Write(WriteError),
}

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
        let mut output = BytesOutputQueue::new();
        writable.write(&mut output).map_err(|err| WriteObjectError::Write(err))?;
        self.write_bytes(output.get_bytes_vec()).await
            .map_err(|err| WriteObjectError::Send(err))
    }

    pub async fn close(&self) -> Result<(), SendError<ConnectionMessage>> {
        self.sender.send(ConnectionMessage::Close).await
    }
}

impl<H: ProtocolServerHandler<R>, R: PacketNode + Send + Sync> ProtocolServer<H, R> {
    pub async fn run(self) -> tokio::io::Result<()> {
        let server = Arc::new(self);
        let listener = TcpListener::bind(&server.host).await?;
        loop {
            let (stream, addr) = match listener.accept().await {
                Ok((stream, addr)) => (stream, addr),
                Err(e) => {
                    error!("Failed to accept client: {}", e);
                    continue;
                }
            };
            let (connection, receiver) = Connection::new(addr);
            let connection = Arc::new(connection);
            let (read_half, write_half) = stream.into_split();
            ProtocolServer::read(server.clone(), read_half, connection.clone());
            ProtocolServer::write(server.clone(), write_half, connection.clone(), receiver);
        };
    }

    async fn read(server: Arc<ProtocolServer<H, R>>, read_half: OwnedReadHalf, connection: Arc<Connection>) {
        let mut state = State::Handshake;
        server.handler.handle_connect(connection.clone());
        let mut input_queue =
            match ProtocolServerInputQueue::<BUFFER_SIZE>::new(read_half).await {
                Ok(res) => res,
                Err(_) => {
                    connection.close();
                    return;
                }
            };
        loop {
            match R::read(state, &mut input_queue).await {
                Ok(packet) => server.handler.handle_event(connection.clone(), &mut state, packet),
                Err(_) => {
                    connection.close();
                    return;
                }
            };
        }
    }

    async fn write(server: Arc<ProtocolServer<H, R>>, mut write_half: OwnedWriteHalf,
                   connection: Arc<Connection>, mut receiver: Receiver<ConnectionMessage>) {
        while let Some(message) = receiver.recv().await {
            match message {
                ConnectionMessage::Bytes(bytes) =>
                    if let Err(err) = write_half.write_all(bytes.as_slice()).await {
                        warn!("Failed to write bytes: {:?}", err);
                    },
                ConnectionMessage::Close => {
                    write_half.shutdown().await;
                    server.handler.handle_disconnect(connection);
                    break;
                },
            }
        }
    }
}