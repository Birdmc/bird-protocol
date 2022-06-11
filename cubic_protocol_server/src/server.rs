use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::{TcpListener, TcpStream};
use tokio::*;
use cubic_protocol::packet::PacketState;
use crate::connection::Connection;
use crate::handler::{ConnectionHandler, ReadHandler};
use crate::read::ReadStreamQueue;
use crate::write::WriteStreamQueue;


pub struct ProtocolServerDeclare<
    H: ReadHandler + Sized + Send + Sync + 'static,
    C: ConnectionHandler + Sized + Send + Sync + 'static,
> {
    pub host: String,
    pub read_handler: H,
    pub connection_handler: C,
}

pub struct ProtocolServerRuntime {
    pub running: AtomicBool,
}

pub struct ProtocolServerTask {
    pub runtime: Arc<ProtocolServerRuntime>,
    pub task: task::JoinHandle<io::Result<()>>,
}

pub fn run_server<
    H: ReadHandler + Sized + Send + Sync + 'static,
    C: ConnectionHandler + Sized + Send + Sync + 'static
>(declare: ProtocolServerDeclare<H, C>) -> ProtocolServerTask {
    let runtime = Arc::new(
        ProtocolServerRuntime {
            running: AtomicBool::new(true),
        }
    );
    let task_runtime = runtime.clone();
    ProtocolServerTask {
        task: tokio::spawn(async move {
            run_server_runtime(declare, task_runtime).await
        }),
        runtime,
    }
}

const CHANNEL_BUFFER_SIZE: usize = 128;
const READ_BUFFER_SIZE: usize = 1024;

async fn run_server_runtime<
    H: ReadHandler + Sized + Send + Sync + 'static,
    C: ConnectionHandler + Sized + Send + Sync + 'static
>(declare: ProtocolServerDeclare<H, C>, runtime: Arc<ProtocolServerRuntime>) -> io::Result<()> {
    let declare = Arc::new(declare);
    let listener = TcpListener::bind(&declare.host).await?;
    while runtime.running.load(Ordering::Acquire) {
        let (stream, addr) = listener.accept().await?;
        let declare = declare.clone();
        tokio::spawn(async move { connection_run(declare, stream, addr).await });
    }
    Ok(())
}

async fn connection_run<
    H: ReadHandler + Sized + Send + Sync + 'static,
    C: ConnectionHandler + Sized + Send + Sync + 'static
>(declare: Arc<ProtocolServerDeclare<H, C>>, stream: TcpStream, addr: SocketAddr) {
    let (read_half, write_half) = stream.into_split();
    let (sender, receiver) =
        sync::mpsc::channel(CHANNEL_BUFFER_SIZE);
    let connection = Arc::new(Connection::new(addr, sender));
    let mut read_queue = ReadStreamQueue::<READ_BUFFER_SIZE>::new(read_half);
    {
        let write_queue = WriteStreamQueue { write_half, receiver };
        let connection = connection.clone();
        let declare = declare.clone();
        tokio::spawn(async move { write_queue.run(connection, declare).await });
    }
    let mut state = PacketState::Handshake;
    loop {
        if let Err(_) = read_queue.next_packet().await {
            let _ = connection.close().await;
            break;
        }
        if let Err(_) = declare.read_handler.handle(
            connection.clone(), &mut state, &mut read_queue).await {
            let _ = connection.close().await;
            break;
        }
    }
}