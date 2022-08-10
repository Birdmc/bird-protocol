use std::borrow::Cow;
use std::marker::PhantomData;
use anyhow::Error;
use uuid::Uuid;
use crate::Packet;
use crate::packet::{PacketRead, PacketReadable, PacketReadableError, PacketWritable, PacketVariantReadable, PacketVariantWritable, PacketWrite};

pub struct VarInt;

pub struct VarLong;

pub trait ProtocolArray {}

pub struct RemainingSlice<
    Value,
    ValueInner = Value,
>(
    PhantomData<Value>, PhantomData<ValueInner>,
);

pub struct RemainingBytesSlice;

pub struct LengthProvidedSlice<
    Length,
    Value,
    LengthInner: PacketLength = Length,
    ValueInner = Value
>(
    PhantomData<Length>, PhantomData<LengthInner>, PhantomData<Value>, PhantomData<ValueInner>,
);

pub struct LengthProvidedBytesSlice<
    Length,
    LengthInner: PacketLength = Length
>(
    PhantomData<Length>,
    PhantomData<LengthInner>,
);

pub struct ProtocolJson;

pub struct ProtocolNbt;

#[repr(C)]
pub struct BlockPosition {
    pub x: i32,
    pub y: i16,
    pub z: i32,
}

pub struct Angle;

/// Packet variant for PacketWritable and PacketReadable.
///
/// Writable: Write packet id as [VarInt] and then packet itself.
///
/// Readable: Just read packet.
pub struct PacketVariant;

impl ProtocolArray for RemainingBytesSlice {}

impl<
    Value,
    ValueInner
> ProtocolArray for RemainingSlice<Value, ValueInner> {}

impl<
    Length,
    Value,
    LengthInner: PacketLength,
    ValueInner
> ProtocolArray for LengthProvidedSlice<Length, Value, LengthInner, ValueInner> {}

impl<
    Length,
    LengthInner: PacketLength
> ProtocolArray for LengthProvidedBytesSlice<Length, LengthInner> {}

impl<'a> PacketReadable<'a> for u8 {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        read.take_byte()
    }
}

impl PacketWritable for u8 {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        write.write_byte(*self)
    }
}

impl<'a> PacketReadable<'a> for i8 {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        u8::read(read).map(|val| val as i8)
    }
}

impl PacketWritable for i8 {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        write.write_byte(*self as u8)
    }
}

impl<'a> PacketReadable<'a> for bool {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        u8::read(read).map(|val| val != 0)
    }
}

impl PacketWritable for bool {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        match self {
            true => 1u8,
            false => 0u8
        }.write(write)
    }
}

fn read_str_with_limit<'a, R>(read: &mut R, limit: i32) -> Result<&'a str, PacketReadableError>
    where R: PacketRead<'a> {
    let slice = read_bytes_with_limit(read, limit)?;
    std::str::from_utf8(slice).map_err(|err| PacketReadableError::Any(err.into()))
}

fn read_bytes_with_limit<'a, R>(read: &mut R, limit: i32) -> Result<&'a [u8], PacketReadableError>
    where R: PacketRead<'a> {
    let length = VarInt::read_variant(read)?;
    match length > limit {
        true => Err(PacketReadableError::Any(anyhow::Error::msg("Too big string"))),
        false => {
            Ok(read.take_slice(length as usize)?)
        }
    }
}

const DEFAULT_LIMIT: i32 = 32767;
const CHAT_LIMIT: i32 = 262144;

impl<'a> PacketReadable<'a> for &'a str {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        read_str_with_limit(read, DEFAULT_LIMIT)
    }
}

impl<'a> PacketReadable<'a> for String {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        <&'_ str>::read(read).map(|str| str.into())
    }
}

impl<'a> PacketReadable<'a> for Cow<'a, str> {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        Ok(Cow::Borrowed(<&'a str>::read(read)?))
    }
}

impl PacketWritable for &str {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        match self.len() > DEFAULT_LIMIT as usize {
            true => Err(Error::msg("Too big string")),
            false => LengthProvidedBytesSlice::<VarInt, i32>::write_variant(self.as_bytes(), write)
        }
    }
}

impl PacketWritable for String {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        self.as_str().write(write)
    }
}

impl PacketWritable for Cow<'_, str> {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        match self {
            Cow::Owned(str) => str.write(write),
            Cow::Borrowed(str) => str.write(write)
        }
    }
}

impl<'a, V: 'a + Clone, T: PacketVariantReadable<'a, &'a [V]> + ProtocolArray> PacketVariantReadable<'a, Vec<V>> for T {
    fn read_variant<R>(read: &mut R) -> Result<Vec<V>, PacketReadableError> where R: PacketRead<'a> {
        Ok(T::read_variant(read)?.to_vec())
    }
}

impl<'a, V: 'a + Clone, T: PacketVariantReadable<'a, &'a [V]> + ProtocolArray> PacketVariantReadable<'a, Cow<'a, [V]>> for T {
    fn read_variant<R>(read: &mut R) -> Result<Cow<'a, [V]>, PacketReadableError> where R: PacketRead<'a> {
        Ok(Cow::Borrowed(T::read_variant(read)?))
    }
}

impl<V, T: PacketVariantWritable<[V]> + ProtocolArray> PacketVariantWritable<Vec<V>> for T {
    fn write_variant<W>(object: &Vec<V>, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        T::write_variant(object.as_slice(), write)
    }
}

impl<V: Clone, T: PacketVariantWritable<[V]> + ProtocolArray> PacketVariantWritable<Cow<'_, [V]>> for T {
    fn write_variant<W>(object: &Cow<'_, [V]>, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        match object {
            Cow::Owned(ref owned) => T::write_variant(owned.as_slice(), write),
            Cow::Borrowed(ref borrowed) => T::write_variant(borrowed, write),
        }
    }
}

impl<'a> PacketVariantReadable<'a, &'a [u8]> for RemainingBytesSlice {
    fn read_variant<R>(read: &mut R) -> Result<&'a [u8], PacketReadableError> where R: PacketRead<'a> {
        read.take_slice(read.available())
    }
}

impl PacketVariantWritable<&[u8]> for RemainingBytesSlice {
    fn write_variant<W>(object: &&[u8], write: &mut W) -> Result<(), Error> where W: PacketWrite {
        write.write_bytes(object)
    }
}

impl PacketVariantWritable<Vec<u8>> for RemainingBytesSlice {
    fn write_variant<W>(object: &Vec<u8>, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        write.write_bytes(object.as_slice())
    }
}

impl<
    'a,
    Value: PacketVariantReadable<'a, ValueInner>,
    ValueInner: 'a
> PacketVariantReadable<'a, Vec<ValueInner>> for RemainingSlice<Value, ValueInner> {
    fn read_variant<R>(read: &mut R) -> Result<Vec<ValueInner>, PacketReadableError> where R: PacketRead<'a> {
        let mut result = Vec::new();
        while read.available() != 0 {
            result.push(Value::read_variant(read)?);
        }
        Ok(result)
    }
}

impl<
    Value: PacketVariantWritable<ValueInner>,
    ValueInner
> PacketVariantWritable<[ValueInner]> for RemainingSlice<Value, ValueInner> {
    fn write_variant<W>(object: &[ValueInner], write: &mut W) -> Result<(), Error> where W: PacketWrite {
        for element in object {
            Value::write_variant(element, write)?
        }
        Ok(())
    }
}

pub trait PacketLength {
    fn into_length(self) -> usize;

    fn from_length(length: usize) -> Self;
}

impl<
    'a,
    Length: PacketVariantReadable<'a, LengthInner>,
    LengthInner: PacketLength
> PacketVariantReadable<'a, &'a [u8]> for LengthProvidedBytesSlice<Length, LengthInner> {
    fn read_variant<R>(read: &mut R) -> Result<&'a [u8], PacketReadableError> where R: PacketRead<'a> {
        let length = Length::read_variant(read)?.into_length();
        read.take_slice(length)
    }
}

impl<
    Length: PacketVariantWritable<LengthInner>,
    LengthInner: PacketLength
> PacketVariantWritable<[u8]> for LengthProvidedBytesSlice<Length, LengthInner> {
    fn write_variant<W>(object: &[u8], write: &mut W) -> Result<(), Error> where W: PacketWrite {
        Length::write_variant(&LengthInner::from_length(object.len()), write)?;
        write.write_bytes(object)
    }
}

impl<
    'a,
    Length: PacketVariantReadable<'a, LengthInner>,
    Value: PacketVariantReadable<'a, ValueInner>,
    LengthInner: PacketLength,
    ValueInner: 'a
> PacketVariantReadable<'a, Vec<ValueInner>> for LengthProvidedSlice<Length, Value, LengthInner, ValueInner> {
    fn read_variant<R>(read: &mut R) -> Result<Vec<ValueInner>, PacketReadableError> where R: PacketRead<'a> {
        let length = Length::read_variant(read)?.into_length();
        let mut result = Vec::with_capacity(length);
        for _ in 0..length {
            result.push(Value::read_variant(read)?);
        }
        Ok(result)
    }
}

impl<
    Length: PacketVariantWritable<LengthInner>,
    Value: PacketVariantWritable<ValueInner>,
    LengthInner: PacketLength,
    ValueInner
> PacketVariantWritable<[ValueInner]> for LengthProvidedSlice<Length, Value, LengthInner, ValueInner> {
    fn write_variant<W>(object: &[ValueInner], write: &mut W) -> Result<(), Error> where W: PacketWrite {
        Length::write_variant(&LengthInner::from_length(object.len()), write)?;
        for element in object {
            Value::write_variant(element, write)?
        }
        Ok(())
    }
}

impl<'a, T: 'a + serde::Deserialize<'a>> PacketVariantReadable<'a, T> for ProtocolJson {
    fn read_variant<R>(read: &mut R) -> Result<T, PacketReadableError> where R: PacketRead<'a> {
        let slice = read_bytes_with_limit(read, DEFAULT_LIMIT)?;
        serde_json::from_slice(slice).map_err(|err| PacketReadableError::Any(err.into()))
    }
}

impl<T: serde::Serialize> PacketVariantWritable<T> for ProtocolJson {
    fn write_variant<W>(object: &T, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        let bytes = serde_json::to_vec(object)?;
        match bytes.len() > DEFAULT_LIMIT as usize {
            true => Err(Error::msg("Too big json")),
            false => LengthProvidedBytesSlice::<VarInt, i32>::write_variant(
                &bytes, write,
            )
        }
    }
}

impl<'a> PacketVariantReadable<'a, f32> for Angle {
    fn read_variant<R>(read: &mut R) -> Result<f32, PacketReadableError> where R: PacketRead<'a> {
        Ok(u8::read(read)? as f32 * std::f32::consts::PI / 256f32)
    }
}

impl PacketVariantWritable<f32> for Angle {
    fn write_variant<W>(object: &f32, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        // TODO should we check for panic situations?
        ((*object * 256f32 / std::f32::consts::PI) as u8).write(write)
    }
}

impl<'a> PacketReadable<'a> for BlockPosition {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        let value = u64::read(read)?;
        let mut x = (value >> 38) as i32;
        let mut y = (value & 0xFFF) as i16;
        let mut z = ((value >> 12) & 0x3FFFFFF) as i32;
        if x >= 0x2000000 {
            x -= 0x4000000
        }
        if y >= 0x800 {
            y -= 0x1000
        }
        if z >= 0x2000000 {
            z -= 0x4000000
        }
        Ok(BlockPosition { x, y, z })
    }
}

impl PacketWritable for BlockPosition {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        (
            ((self.x as i64 & 0x3FFFFFF) << 38) |
                ((self.z as i64 & 0x3FFFFFF) << 12) |
                (self.y as i64 & 0xFFF)
        ).write(write)
    }
}

impl<'a> PacketReadable<'a> for bird_chat::component::Component<'a> {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        let bytes = read_bytes_with_limit(read, CHAT_LIMIT)?;
        serde_json::from_slice(bytes).map_err(|err| PacketReadableError::Any(err.into()))
    }
}

impl PacketWritable for bird_chat::component::Component<'_> {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        let bytes = serde_json::to_vec(self)?;
        match bytes.len() > CHAT_LIMIT as usize {
            true => Err(Error::msg("Too big component json")),
            false => LengthProvidedBytesSlice::<VarInt, i32>::write_variant(&bytes, write)
        }
    }
}

impl<'a> PacketReadable<'a> for bird_chat::identifier::Identifier<'a> {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        bird_chat::identifier::Identifier::new_fulled(<&'a str>::read(read)?)
            .map_err(|err| PacketReadableError::Any(err.into()))
    }
}

impl PacketWritable for bird_chat::identifier::Identifier<'_> {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        self.get_fulled().write(write)
    }
}

impl<'a> PacketReadable<'a> for Uuid {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        Uuid::from_slice(read.take_slice(16)?)
            .map_err(|err| PacketReadableError::Any(err.into()))
    }
}

impl PacketWritable for Uuid {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        write.write_bytes(self.as_bytes().as_slice())
    }
}

impl<'a, T: PacketReadable<'a> + Packet> PacketVariantReadable<'a, T> for PacketVariant {
    fn read_variant<R>(read: &mut R) -> Result<T, PacketReadableError> where R: PacketRead<'a> {
        T::read(read)
    }
}

impl<T: PacketWritable + Packet> PacketVariantWritable<T> for PacketVariant {
    fn write_variant<W>(object: &T, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        VarInt::write_variant(&T::id(), write)?;
        T::write(object, write)
    }
}

macro_rules! length_impl {
    ($num: ident) => {
        impl const PacketLength for $num {
            fn into_length(self) -> usize {
                self as usize
            }

            fn from_length(length: usize) -> Self {
                length as Self
            }
        }
    };
    ($($num: ident$(,)*)*) => {
        $(length_impl!($num);)*
    }
}

macro_rules! number_impl {
    ($num: ident) => {
        impl<'a> PacketReadable<'a> for $num {
            fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
                let mut bytes = [0u8; std::mem::size_of::<Self>()];
                let slice = read.take_slice(bytes.len())?;
                unsafe {
                    // Safety. Slice reference is valid, bytes reference also. They don't overlap
                    std::ptr::copy_nonoverlapping(slice.as_ptr(), bytes.as_mut_ptr(), bytes.len())
                }
                Ok(Self::from_be_bytes(bytes))
            }
        }

        impl PacketWritable for $num {
            fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
                write.write_bytes_fixed(self.to_be_bytes())
            }
        }
    };
    ($($num:ident$(,)*)*) => {
        $(number_impl!($num);)*
    }
}

macro_rules! var_number_impl {
    ($var_num: ident, $num: ident, $unsigned_num: ident) => {
        impl<'a> PacketVariantReadable<'a, $num> for $var_num {
            fn read_variant<R>(read: &mut R) -> Result<$num, PacketReadableError> where R: PacketRead<'a> {
                let mut value: $num = 0;
                let mut position: u8 = 0;
                loop {
                    let byte = read.take_byte()?;
                    value |= ((byte & 0x7F) as $num) << position;
                    if (byte & 0x80) == 0 {
                        break Ok(value)
                    }
                    position += 7;
                    if position >= (std::mem::size_of::<$num>() * 8) as u8 {
                        break Err(PacketReadableError::Any(anyhow::Error::msg("Var number is too long")))
                    }
                }
            }
        }

        impl PacketVariantWritable<$num> for $var_num {
            fn write_variant<W>(object: & $num, write: &mut W) -> Result<(), Error> where W: PacketWrite {
                let mut value: $unsigned_num = *object as $unsigned_num;
                loop {
                    if (value & !0x7F) == 0 {
                        write.write_byte((value as u8))?;
                        break;
                    }
                    write.write_byte(((value & 0x7F) | 0x80) as u8)?;
                    value >>= 7;
                }
                Ok(())
            }
        }

    }
}

length_impl!(u8 i8 u16 i16 u32 i32 u64 i64);
number_impl!(u16 i16 u32 i32 u64 i64 u128 i128);
var_number_impl!(VarInt, i32, u32);
var_number_impl!(VarLong, i64, u64);