use cubic_chat::component::ComponentType;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusVersion {
    pub name: String,
    pub protocol: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusSample {
    pub name: String,
    pub id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusPlayers {
    pub max: i32,
    pub online: i32,
    pub sample: Vec<StatusSample>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusResponse {
    pub version: StatusVersion,
    pub players: StatusPlayers,
    pub description: ComponentType,
    pub favicon: String,
}