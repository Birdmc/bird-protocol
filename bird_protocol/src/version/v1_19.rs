use uuid::Uuid;
use crate::types::*;
use bird_protocol_derive::{Packet, PacketWritable, PacketReadable};

pub const PROTOCOL: i32 = 757;

pub type VelocityType = euclid::default::Vector3D<i16>;
pub type CoordinateType = euclid::default::Vector3D<f64>;

#[derive(PacketWritable, PacketReadable, Debug, Clone)]
pub(crate) struct VelocityWrapper {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

impl From<VelocityWrapper> for VelocityType {
    fn from(velocity: VelocityWrapper) -> Self {
        Self::new(velocity.x, velocity.y, velocity.z)
    }
}

impl From<&VelocityType> for VelocityWrapper {
    fn from(velocity: &VelocityType) -> Self {
        Self {
            x: velocity.x,
            y: velocity.y,
            z: velocity.z,
        }
    }
}

#[derive(PacketWritable, PacketReadable, Debug, Clone)]
pub(crate) struct CoordinateWrapper {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl From<CoordinateWrapper> for CoordinateType {
    fn from(wrapper: CoordinateWrapper) -> Self {
        Self::new(wrapper.x, wrapper.y, wrapper.z)
    }
}

impl From<&CoordinateType> for CoordinateWrapper {
    fn from(wrapper: &CoordinateType) -> Self {
        Self {
            x: wrapper.x,
            y: wrapper.y,
            z: wrapper.z,
        }
    }
}

#[derive(Packet, Clone, PartialEq, Debug)]
#[packet(id = 0x00, state = Play, side = Server, protocol = PROTOCOL)]
pub struct AddEntity {
    #[pf(variant = VarInt)]
    pub entity_id: i32,
    pub object_uuid: Uuid,
    #[pf(variant = VarInt)]
    pub entity_type: i32,
    #[pf(variant = CoordinateWrapper)]
    pub coordinates: CoordinateType,
    pub pitch: Angle,
    pub yaw: Angle,
    pub head_yaw: Angle,
    #[pf(variant = VarInt)]
    pub data: i32,
    #[pf(variant = VelocityWrapper)]
    pub velocity: VelocityType,
}

#[derive(Packet, Clone, PartialEq, Debug)]
#[packet(id = 0x01, state = Play, side = Server, protocol = PROTOCOL)]
pub struct AddExperienceOrb {
    #[pf(variant = VarInt)]
    pub entity_id: i32,
    #[pf(variant = CoordinateWrapper)]
    pub coordinates: CoordinateType,
    pub count: i16,
}

#[derive(Packet, Clone, PartialEq, Debug)]
#[packet(id = 0x02, state = Play, side = Server, protocol = PROTOCOL)]
pub struct AddPlayer {
    #[pf(variant = VarInt)]
    pub entity_id: i32,
    pub player_uuid: Uuid,
    #[pf(variant = CoordinateWrapper)]
    pub coordinates: CoordinateType,
    pub yaw: Angle,
    pub pitch: Angle,
}

#[derive(PacketWritable, PacketReadable, Copy, Clone, PartialEq, Debug)]
#[pe(primitive = u8)]
pub enum Animation {
    SwingMainArm,
    TakeDamage,
    LeaveBed,
    SwingOffHand,
    CriticalEffect,
    MagicCriticalEffect,
}

#[derive(Packet, Clone, PartialEq, Debug)]
#[packet(id = 0x03, state = Play, side = Server, protocol = PROTOCOL)]
pub struct Animate {
    #[pf(variant = VarInt)]
    pub entity_id: i32,
    pub animation: Animation,
}