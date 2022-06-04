use macros::{PacketWritable, Packet, PacketReadable};

#[derive(PacketWritable, Packet, PacketReadable)]
#[packet(id = 0x00, side = Server, state = Handshake, protocol = 0)]
pub struct Handshaking {
    #[pf(variant = VarInt)]
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    #[pf(variant = VarInt)]
    pub next_state: i32,
}