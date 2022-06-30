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
    pub value: Vec<T>,
}

impl<T> RemainingBytesArray<T> {
    pub fn new(value: Vec<T>) -> Self {
        Self { value }
    }

    pub fn get(&self) -> &Vec<T> {
        &self.value
    }
}

impl<T> From<Vec<T>> for RemainingBytesArray<T> {
    fn from(value: Vec<T>) -> Self {
        RemainingBytesArray::new(value)
    }
}

impl<T> From<RemainingBytesArray<T>> for Vec<T> {
    fn from(array: RemainingBytesArray<T>) -> Self {
        array.value
    }
}

pub struct LengthProvidedArray<T, S> {
    pub value: Vec<T>,
    size: PhantomData<S>,
}

impl<T, S> LengthProvidedArray<T, S> {
    pub fn new(value: Vec<T>) -> Self {
        Self { value, size: PhantomData }
    }

    pub fn get(&self) -> &Vec<T> {
        &self.value
    }
}

impl<T, S> From<Vec<T>> for LengthProvidedArray<T, S> {
    fn from(value: Vec<T>) -> Self {
        LengthProvidedArray::new(value)
    }
}

impl<T, S> From<LengthProvidedArray<T, S>> for Vec<T> {
    fn from(array: LengthProvidedArray<T, S>) -> Self {
        array.value
    }
}

impl<T> ProtocolJson<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn into(self) -> T {
        self.value
    }
}

impl<T> From<T> for ProtocolJson<T> {
    fn from(val: T) -> Self {
        ProtocolJson::new(val)
    }
}

impl<T> ProtocolNbt<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn int(self) -> T {
        self.value
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
