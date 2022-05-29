use std::ops::Not;
use crate::bytes::{InputByteQueue, OutputByteQueue};
use crate::protocol::{Readable, ReadError, Writable, WriteError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bound {
    Server,
    Client,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Handshake,
    Status,
    Login,
    Play,
}

pub trait Packet: Writable + Readable {
    fn id() -> i32;

    fn bound() -> Bound;

    fn state() -> State;

    fn protocol() -> i32;
}

#[async_trait::async_trait]
pub trait PacketNode: Sized {
    async fn read(state: State, input: &mut impl InputByteQueue) -> Result<Self, ReadError>;
}

#[derive(Debug, Default)]
pub struct EntityNothing {}

impl Writable for EntityNothing {
    fn write(&self, _: &mut impl OutputByteQueue) -> Result<(), WriteError> {
        Ok(())
    }
}