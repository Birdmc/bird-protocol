use std::borrow::Cow;
use bird_chat::component::Component;
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

#[derive(PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
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

#[derive(PacketWritable, PacketReadable, Debug, Clone, PartialEq)]
pub struct StatusResponse<'a>(
    #[variant(ProtocolJson)]
    pub StatusResponseObject<'a>,
);

fn is_cow_empty<T: Clone>(cow: &Cow<[T]>) -> bool {
    cow.is_empty()
}