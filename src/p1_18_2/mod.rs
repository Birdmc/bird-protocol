use crate::protocol::*;
use crate::version::*;
use crate::bytes::*;
use crate::*;
use crate::status::*;
use cubic_chat::component::*;
use cubic_chat::identifier::*;
use uuid::Uuid;

protocol_enum! {
    VarInt, NextState {
        Status => 1,
        Login => 2,
    }
}

protocol_packets! {
    757, 1_18_2 => {
        Handshake {
            Client {
            }
            Server {
                0x00, Handshaking {
                    protocol_version: VarInt,
                    server_address: String,
                    server_port: u16,
                    next_state: NextState,
                }
            }
        }
        Status {
            Client {
                0x00, Response {
                    response: ProtocolJson<StatusResponse>,
                }
                0x01, Pong {
                    payload: i64,
                }
            }
            Server {
                0x00, Request {
                }
                0x01, Ping {
                    payload: i64,
                }
            }
        }
        Login {
            Client {
                0x00, Disconnect {
                    reason: ComponentType,
                }
                0x01, EncryptionRequest {
                    server_id: String,
                    public_key: LengthProvidedArray<u8, VarInt>,
                    verify_token: LengthProvidedArray<u8, VarInt>,
                }
                0x02, LoginSuccess {
                    uuid: Uuid,
                    username: String,
                }
                0x03, SetCompression {
                    threshold: VarInt,
                }
                0x04, LoginPluginRequest {
                    message_id: VarInt,
                    channel: Identifier,
                    data: RemainingBytesArray<u8>,
                }
            }
            Server {
                0x00, LoginStart {
                    name: String,
                }
                0x01, EncryptionResponse {
                    shared_secret: LengthProvidedArray<u8, VarInt>,
                    verify_token: LengthProvidedArray<u8, VarInt>,
                }
                0x02, LoginPluginResponse {
                    message_id: VarInt,
                    successful: bool,
                    data: RemainingBytesArray<u8>,
                }
            }
        }
        Play {
            Client {
            }
            Server {
            }
        }
    }
}

protocol_enum! {
    VarInt, Direction {
        Down => 0,
        Up => 1,
        North => 2,
        South => 3,
        West => 4,
        East => 5,
    }
}

entity_data! {
    Data {
        0 => u8,
        1 => VarInt,
        2 => f32,
        3 => String,
        4 => ComponentType,
        5 => Option<ComponentType>,
        // 6 => Slot,
        7 => bool,
        8 => Rotation,
        9 => BlockPosition,
        10 => Option<BlockPosition>,
        11 => Direction,
        12 => Option<Uuid>,
        13 => BlockId,
    }
    Entities {

    }
}