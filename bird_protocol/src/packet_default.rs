use crate::*;

#[derive(PacketWritable, PacketReadable)]
pub struct HandshakePacket<'a> {
    #[variant(VarInt)]
    pub protocol_version: i32,
    pub server_address: &'a str,
    pub server_port: u16,
    #[variant(VarInt)]
    pub next_state: i32,
}