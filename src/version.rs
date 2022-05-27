use crate::bytes::{InputByteQueue};
use crate::protocol::{Readable, ReadError, Writable};

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

pub mod entity {
    use crate::bytes::{InputByteQueue, OutputByteQueue};
    use crate::protocol::{Readable, ReadError, VarInt, Writable, WriteError};

    pub struct EntityDataEntry<T: Sized> {
        pub index: u8,
        pub value_type_id: i32,
        pub value: T,
    }

    impl<T: Writable + Sized> Writable for EntityDataEntry<T> {
        fn write(&self, output: &mut impl OutputByteQueue) -> Result<(), WriteError> {
            self.index.write(output)?;
            VarInt(self.value_type_id).write(output)?;
            self.value.write(output)
        }
    }

    impl<T: Readable + Sized> EntityDataEntry<T> {
        pub async fn read(input: &mut impl InputByteQueue, index: u8, value_type_id: i32) -> Result<Self, ReadError> {
            Ok(EntityDataEntry {
                index, value_type_id,
                value: T::read(input).await?
            })
        }

        pub async fn read_value_type_id(input: &mut impl InputByteQueue) -> Result<i32, ReadError> {
            VarInt::read(input).await.map(|val| val.into())
        }

        pub fn is_end(index: u8) -> bool {
            index == 0xff
        }
    }

}