use std::marker::PhantomData;
use euclid::default::Vector3D;

pub type Angle = euclid::Angle<f32>;
pub type BlockPosition = Vector3D<i32>;

pub struct ReadRemainingBytesArray<T> {
    pub value: Vec<T>,
}

pub struct WriteRemainingBytesArray<'a, T> {
    pub value: &'a Vec<T>,
}

impl<T> ReadRemainingBytesArray<T> {
    pub fn new(value: Vec<T>) -> Self {
        Self { value }
    }
}

impl<T> From<ReadRemainingBytesArray<T>> for Vec<T> {
    fn from(value: ReadRemainingBytesArray<T>) -> Self {
        value.value
    }
}

impl<'a, T> From<&'a Vec<T>> for WriteRemainingBytesArray<'a, T> {
    fn from(value: &'a Vec<T>) -> Self {
        Self { value }
    }
}

pub struct ReadLengthProvidedArray<T, S> {
    pub value: Vec<T>,
    size: PhantomData<S>,
}

pub struct WriteLengthProvidedArray<'a, T, S> {
    pub value: &'a Vec<T>,
    size: PhantomData<S>,
}

impl<T, S> ReadLengthProvidedArray<T, S> {
    pub fn new(value: Vec<T>) -> Self {
        Self { value, size: PhantomData }
    }
}

impl<T, S> From<ReadLengthProvidedArray<T, S>> for Vec<T> {
    fn from(value: ReadLengthProvidedArray<T, S>) -> Self {
        value.value
    }
}

impl<'a, T, S> From<&'a Vec<T>> for WriteLengthProvidedArray<'a, T, S> {
    fn from(value: &'a Vec<T>) -> Self {
        Self { value, size: PhantomData }
    }
}

macro_rules! advanced_container_type {
    ($write: ident, $read: ident) => {
        pub struct $read<T> {
            pub value: T
        }

        pub struct $write<'a, T> {
            pub value: &'a T
        }


        impl<T> $read<T> {
            pub fn into(self) -> T {
                self.value
            }
        }

        impl<'a, T> $write<'a, T> {
            pub fn from(value: &'a T) -> Self {
                Self { value }
            }
        }
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

        impl From<& $contained> for $name {
            fn from(value: & $contained) -> Self {
                $name(*value)
            }
        }
    }
}

advanced_container_type!(WriteProtocolJson, ReadProtocolJson);
advanced_container_type!(WriteProtocolNbt, ReadProtocolNbt);
container_type!(VarInt, i32);
container_type!(VarLong, i64);
