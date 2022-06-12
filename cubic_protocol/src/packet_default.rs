use cubic_chat::component::ComponentType;
use cubic_chat::identifier::Identifier;
use uuid::Uuid;
use cubic_protocol_derive::{Packet, PacketWritable, PacketReadable};
use serde::{Serialize, Deserialize};
use crate::packet_node;
use crate::types::*;

type RemainingBytesArrayU8 = RemainingBytesArray<u8>;
type LengthProvidedArrayU8VarInt = LengthProvidedArray<u8, VarInt>;

#[derive(Packet, Debug)]
#[packet(id = 0x00, side = Client, state = Handshake, protocol = 0)]
pub struct Handshaking {
    #[pf(variant = VarInt)]
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    #[pf(variant = VarInt)]
    pub next_state: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponseVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponseSample {
    pub name: String,
    pub uuid: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponsePlayers {
    pub max: i32,
    pub online: i32,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    pub sample: Vec<StatusResponseSample>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StatusResponseDescription {
    Component(ComponentType),
    String(String),
}

impl Default for StatusResponseDescription {
    fn default() -> Self {
        StatusResponseDescription::String("".into())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponseObject {
    pub version: StatusResponseVersion,
    pub players: StatusResponsePlayers,
    #[serde(default = "Default::default")]
    pub description: StatusResponseDescription,
    #[serde(skip_serializing_if = "String::is_empty", default = "String::new")]
    pub favicon: String,
}

type StatusResponseObjectJson = ProtocolJson<StatusResponseObject>;

#[derive(Packet, Debug)]
#[packet(id = 0x00, side = Server, state = Status, protocol = 0)]
pub struct StatusResponse {
    #[pf(variant = StatusResponseObjectJson)]
    pub response: StatusResponseObject,
}

#[derive(Packet, Debug)]
#[packet(id = 0x01, side = Server, state = Status, protocol = 0)]
pub struct StatusPong {
    pub payload: i64,
}

#[derive(Packet, Debug)]
#[packet(id = 0x00, side = Client, state = Status, protocol = 0)]
pub struct StatusRequest;

#[derive(Packet, Debug)]
#[packet(id = 0x01, side = Client, state = Status, protocol = 0)]
pub struct StatusPing {
    pub payload: i64,
}

#[derive(Packet, Debug)]
#[packet(id = 0x00, side = Server, state = Login, protocol = 0)]
pub struct LoginDisconnect {
    pub reason: ComponentType
}

#[derive(PacketWritable, PacketReadable, Debug)]
pub struct SignatureData {
    #[pf(variant = LengthProvidedArrayU8VarInt)]
    pub public_key: Vec<u8>,
    #[pf(variant = LengthProvidedArrayU8VarInt)]
    pub signature: Vec<u8>,
}

#[derive(Packet, Debug)]
#[packet(id = 0x01, side = Server, state = Login, protocol = 0)]
pub struct LoginEncryptionRequest {
    pub server_id: String,
    pub signature_data: SignatureData,
}

#[derive(Packet, Debug)]
#[packet(id = 0x02, side = Server, state = Login, protocol = 0)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
}

#[derive(Packet, Debug)]
#[packet(id = 0x03, side = Server, state = Login, protocol = 0)]
pub struct LoginSetCompression {
    #[pf(variant = VarInt)]
    pub threshold: i32,
}

#[derive(Packet, Debug)]
#[packet(id = 0x04, side = Server, state = Login, protocol = 0)]
pub struct LoginPluginRequest {
    #[pf(variant = VarInt)]
    pub message_id: i32,
    pub channel: Identifier,
    #[pf(variant = RemainingBytesArrayU8)]
    pub data: Vec<u8>,
}

#[derive(Packet, Debug)]
#[packet(id = 0x00, side = Client, state = Login, protocol = 0)]
pub struct LoginStart {
    pub name: String,
    pub signature_data: Option<SignatureData>,
}

#[derive(Packet, Debug)]
#[packet(id = 0x01, side = Client, state = Login, protocol = 0)]
pub struct LoginEncryptionResponse {
    pub signature_data: SignatureData,
}

#[derive(Packet, Debug)]
#[packet(id = 0x02, side = Client, state = Login, protocol = 0)]
pub struct LoginPluginSuccess {
    #[pf(variant = VarInt)]
    pub message_id: i32,
    pub successful: bool,
    #[pf(variant = RemainingBytesArrayU8)]
    pub data: Vec<u8>,
}

packet_node!(#[derive(Debug)] ClientHandshakePacket => [
    Handshaking
]);

packet_node!(#[derive(Debug)] ClientStatusPacket => [
    StatusRequest,
    StatusPing
]);

packet_node!(#[derive(Debug)] ServerStatusPacket => [
    StatusResponse,
    StatusPong
]);

packet_node!(#[derive(Debug)] ServerLoginPacket => [
    LoginDisconnect,
    LoginEncryptionRequest,
    LoginSetCompression,
    LoginPluginRequest
]);

packet_node!(#[derive(Debug)] ClientLoginPacket => [
    LoginStart,
    LoginEncryptionResponse,
    LoginPluginSuccess
]);