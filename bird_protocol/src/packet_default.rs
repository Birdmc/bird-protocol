use std::borrow::Cow;
use bird_chat::component::Component;
use bird_chat::identifier::Identifier;
use uuid::Uuid;
use crate::*;
use serde::{Serialize, Deserialize};

#[derive(PacketWritable, PacketReadable, Debug, Clone, Copy, PartialEq)]
#[enum_type(i32)]
#[enum_variant(VarInt)]
pub enum HandshakeNextState {
    Status = 1,
    Login,
}

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Server, state = Handshake, id = 0x00)]
pub struct HandshakePacket<'a> {
    #[variant(VarInt)]
    pub protocol_version: i32,
    pub server_address: &'a str,
    pub server_port: u16,
    pub next_state: HandshakeNextState,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StatusResponseVersion<'a> {
    pub name: &'a str,
    pub protocol: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StatusResponsePlayers<'a> {
    pub max: i32,
    pub online: i32,
    #[serde(borrow = "'a", skip_serializing_if = "is_cow_empty")]
    pub sample: Cow<'a, [StatusResponseSample<'a>]>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StatusResponseSample<'a> {
    pub name: &'a str,
    pub id: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct StatusResponseObject<'a> {
    pub version: StatusResponseVersion<'a>,
    pub players: StatusResponsePlayers<'a>,
    #[serde(borrow = "'a")]
    pub description: either::Either<&'a str, Component<'a>>,
    #[serde(skip_serializing_if = "str::is_empty")]
    pub favicon: &'a str,
    #[serde(rename = "previewsChat")]
    pub previews_chat: bool,
}

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Client, state = Status, id = 0x00)]
pub struct StatusResponse<'a>(
    #[variant(ProtocolJson)]
    pub StatusResponseObject<'a>,
);

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Client, state = Status, id = 0x01)]
pub struct StatusPingResponse {
    pub payload: i64,
}

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Server, state = Status, id = 0x00)]
pub struct StatusRequest;

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Server, state = Status, id = 0x01)]
pub struct StatusPingRequest {
    pub payload: i64,
}

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Client, state = Login, id = 0x00)]
pub struct LoginDisconnect<'a> {
    pub reason: Component<'a>,
}

type LengthProvidedBytesSliceVI = LengthProvidedBytesSlice<VarInt, i32>;

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Client, state = Login, id = 0x01)]
pub struct LoginEncryptionRequest<'a> {
    pub server_id: &'a str,
    #[variant(LengthProvidedBytesSliceVI)]
    pub public_key: &'a [u8],
    #[variant(LengthProvidedBytesSliceVI)]
    pub verify_token: &'a [u8],
}

#[derive(PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
pub struct LoginSuccessProperty<'a> {
    pub name: &'a str,
    pub value: &'a str,
    pub signature: Option<&'a str>,
}

type LoginSuccessPropertyArray<'a> = LengthProvidedSlice<
    VarInt,
    LoginSuccessProperty<'a>,
    i32,
>;

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Client, state = Login, id = 0x02)]
pub struct LoginSuccess<'a> {
    pub uuid: Uuid,
    pub name: &'a str,
    #[variant(LoginSuccessPropertyArray)]
    pub properties: Cow<'a, [LoginSuccessProperty<'a>]>,
}

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Client, state = Login, id = 0x03)]
pub struct LoginSetCompression {
    #[variant(VarInt)]
    pub threshold: i32,
}

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Client, state = Login, id = 0x04)]
pub struct LoginPluginRequest<'a> {
    #[variant(VarInt)]
    pub message_id: i32,
    pub channel: Identifier<'a>,
    #[variant(RemainingBytesSlice)]
    pub data: &'a [u8],
}

#[derive(PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
pub struct LoginStartSignatureData<'a> {
    pub timestamp: i64,
    #[variant(LengthProvidedBytesSliceVI)]
    pub public_key: &'a [u8],
    #[variant(LengthProvidedBytesSliceVI)]
    pub signature: &'a [u8],
}

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Server, state = Login, id = 0x00)]
pub struct LoginStart<'a> {
    pub name: &'a str,
    pub signature_data: Option<LoginStartSignatureData<'a>>,
}

#[derive(PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[enum_type(u8)]
pub enum LoginEncryptionResponseData<'a> {
    MessageSignature {
        #[variant(LengthProvidedBytesSliceVI)]
        message_signature: &'a [u8]
    },
    VerifyToken {
        #[variant(LengthProvidedBytesSliceVI)]
        verify_token: &'a [u8],
        salt: i64,
    },
}

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Server, state = Login, id = 0x01)]
pub struct LoginEncryptionResponse<'a> {
    #[variant(LengthProvidedBytesSliceVI)]
    pub shared_secret: &'a [u8],
    pub data: LoginEncryptionResponseData<'a>,
}

#[derive(Packet, PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
#[packet(bound = Server, state = Login, id = 0x02)]
pub struct LoginPluginResponse<'a> {
    #[variant(VarInt)]
    pub message_id: i32,
    pub successful: bool,
    #[variant(RemainingBytesSlice)]
    pub data: &'a [u8],
}

fn is_cow_empty<T: Clone>(cow: &Cow<[T]>) -> bool {
    cow.is_empty()
}