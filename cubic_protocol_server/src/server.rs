use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpListener;
use tokio::*;
use crate::write::WriteQueue;

pub struct ProtocolServerDeclare {
    pub host: String,
}

pub struct ProtocolServerRuntime {
    pub running: AtomicBool,
}

pub struct ProtocolServerTask {
    pub runtime: Arc<ProtocolServerRuntime>,
    pub task: task::JoinHandle<io::Result<()>>,
}

pub fn run_server(declare: ProtocolServerDeclare) -> ProtocolServerTask {
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

async fn run_server_runtime(declare: ProtocolServerDeclare, runtime: Arc<ProtocolServerRuntime>) -> io::Result<()> {
    let listener = TcpListener::bind(declare.host).await?;
    while runtime.running.load(Ordering::Acquire) {
        let (stream, addr) = listener.accept().await?;
        let (read_half, write_half) = stream.into_split();
        let (sender, receiver) =
            sync::mpsc::channel(CHANNEL_BUFFER_SIZE);
        let write_queue = WriteQueue { write_half, receiver };
    }
    Ok(())
}

