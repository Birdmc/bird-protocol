use cubic_chat::component::ComponentType;
use cubic_chat::identifier::Identifier;
use uuid::Uuid;
use cubic_protocol_derive::{Packet, PacketWritable, PacketReadable};
use serde::{Serialize, Deserialize};
use crate::{packet_enum, packet_node};
use crate::types::*;

packet_enum! {
    #[derive(Debug, Clone, Copy, PartialEq)] HandshakeNextState, VarInt => {
        Status = 1,
        Login = 2,
    }
}

type RemainingBytesArrayU8 = RemainingBytesArray<u8>;
type LengthProvidedArrayU8VarInt = LengthProvidedArray<u8, VarInt>;

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x00, side = Client, state = Handshake, protocol = 0)]
pub struct Handshaking {
    #[pf(variant = VarInt)]
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: HandshakeNextState,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct StatusResponseVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct StatusResponseSample {
    pub name: String,
    pub id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct StatusResponsePlayers {
    pub max: i32,
    pub online: i32,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "Vec::new")]
    pub sample: Vec<StatusResponseSample>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct StatusResponseObject {
    pub version: StatusResponseVersion,
    pub players: StatusResponsePlayers,
    #[serde(default = "Default::default")]
    pub description: StatusResponseDescription,
    #[serde(skip_serializing_if = "String::is_empty", default = "String::new")]
    pub favicon: String,
}

type StatusResponseObjectJson = ProtocolJson<StatusResponseObject>;

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x00, side = Server, state = Status, protocol = 0)]
pub struct StatusResponse {
    #[pf(variant = StatusResponseObjectJson)]
    pub response: StatusResponseObject,
}

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x01, side = Server, state = Status, protocol = 0)]
pub struct StatusPong {
    pub payload: i64,
}

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x00, side = Client, state = Status, protocol = 0)]
pub struct StatusRequest;

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x01, side = Client, state = Status, protocol = 0)]
pub struct StatusPing {
    pub payload: i64,
}

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x00, side = Server, state = Login, protocol = 0)]
pub struct LoginDisconnect {
    pub reason: ComponentType,
}

#[derive(PacketWritable, PacketReadable, Debug, PartialEq)]
pub struct SignatureData {
    #[pf(variant = LengthProvidedArrayU8VarInt)]
    pub public_key: Vec<u8>,
    #[pf(variant = LengthProvidedArrayU8VarInt)]
    pub signature: Vec<u8>,
}

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x01, side = Server, state = Login, protocol = 0)]
pub struct LoginEncryptionRequest {
    pub server_id: String,
    pub signature_data: SignatureData,
}

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x02, side = Server, state = Login, protocol = 0)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
}

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x03, side = Server, state = Login, protocol = 0)]
pub struct LoginSetCompression {
    #[pf(variant = VarInt)]
    pub threshold: i32,
}

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x04, side = Server, state = Login, protocol = 0)]
pub struct LoginPluginRequest {
    #[pf(variant = VarInt)]
    pub message_id: i32,
    pub channel: Identifier,
    #[pf(variant = RemainingBytesArrayU8)]
    pub data: Vec<u8>,
}

#[derive(PacketWritable, PacketReadable, Debug, PartialEq)]
pub struct LoginStartSignatureData {
    pub timestamp: i64,
    pub data: SignatureData,
}

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x00, side = Client, state = Login, protocol = 0)]
pub struct LoginStart {
    pub name: String,
    pub signature_data: Option<LoginStartSignatureData>,
}

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x01, side = Client, state = Login, protocol = 0)]
pub struct LoginEncryptionResponse {
    pub signature_data: SignatureData,
}

#[derive(Packet, Debug, PartialEq)]
#[packet(id = 0x02, side = Client, state = Login, protocol = 0)]
pub struct LoginPluginSuccess {
    #[pf(variant = VarInt)]
    pub message_id: i32,
    pub successful: bool,
    #[pf(variant = RemainingBytesArrayU8)]
    pub data: Vec<u8>,
}

packet_node!(#[derive(Debug, PartialEq)] ClientHandshakePacket => [
    Handshaking
]);

packet_node!(#[derive(Debug, PartialEq)] ClientStatusPacket => [
    StatusRequest,
    StatusPing
]);

packet_node!(#[derive(Debug, PartialEq)] ServerStatusPacket => [
    StatusResponse,
    StatusPong
]);

packet_node!(#[derive(Debug, PartialEq)] ServerLoginPacket => [
    LoginDisconnect,
    LoginEncryptionRequest,
    LoginSetCompression,
    LoginPluginRequest
]);

packet_node!(#[derive(Debug, PartialEq)] ClientLoginPacket => [
    LoginStart,
    LoginEncryptionResponse,
    LoginPluginSuccess
]);