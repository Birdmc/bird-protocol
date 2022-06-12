/// # Protocol server
/// There exists 4 entities:
/// - Server (Receiving and creating tasks for connections)
/// - Connection (Handling connection closing, packet reading, packet writing)
/// - Packet handler (Handles packets)
/// - Connection handle (Handles connections)

pub mod server;
pub mod write;
pub mod read;
pub mod connection;
pub mod handler;
