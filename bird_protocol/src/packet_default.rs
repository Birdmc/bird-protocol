use crate::*;

#[derive(PacketWritable, PacketReadable)]
#[enum_type(i32)]
#[enum_variant(VarInt)]
pub enum HandshakeNextState {
    Status = 1,
    Login,
}

#[derive(PacketWritable, PacketReadable)]
pub struct HandshakePacket<'a> {
    #[variant(VarInt)]
    pub protocol_version: i32,
    pub server_address: &'a str,
    pub server_port: u16,
    pub next_state: HandshakeNextState,
}