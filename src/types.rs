use std::marker::PhantomData;
use euclid::default::Vector3D;

pub type Angle = euclid::Angle<f32>;
pub type BlockPosition = Vector3D<i32>;

pub struct ProtocolJson<T> {
    pub value: T,
}

pub struct ProtocolNbt<T> {
    pub value: T,
}

pub struct RemainingBytesArray<T> {
    pub result: Vec<T>,
}

pub struct LengthProvidedArray<T, S> {
    pub result: Vec<T>,
    size: PhantomData<S>,
}

impl<T> ProtocolJson<T> {
    pub fn new(value: T) -> ProtocolJson<T> {
        ProtocolJson { value }
    }

    pub fn get(&self) -> &T {
        &self.value
    }
}

impl<T> ProtocolNbt<T> {
    pub fn new(value: T) -> ProtocolNbt<T> {
        ProtocolNbt { value }
    }

    pub fn get(&self) -> &T {
        &self.value
    }
}

macro_rules! container_type {
    ($name: ident, $contained: ty) => {
        #[derive(Debug, Copy, Clone)]
        pub struct $name(pub $contained);

        impl From<$name> for $contained {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl From<$contained> for $name {
            fn from(value: $contained) -> Self {
                $name(value)
            }
        }
    }
}

container_type!(VarInt, i32);
container_type!(VarLong, i64);
