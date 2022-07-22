use std::borrow::Cow;
use std::marker::PhantomData;
use anyhow::Error;
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

pub struct BlockPosition;

pub struct Angle;

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

impl PacketReadable<'_> for u8 {
    fn read(read: &mut PacketRead<'_>) -> Result<Self, PacketReadableError> {
        read.take_byte()
    }
}

impl PacketWritable for u8 {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        write.write_byte(*self)
    }
}

impl PacketReadable<'_> for i8 {
    fn read(read: &mut PacketRead<'_>) -> Result<Self, PacketReadableError> {
        u8::read(read).map(|val| val as i8)
    }
}

impl PacketWritable for i8 {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        write.write_byte(*self as u8)
    }
}

impl PacketReadable<'_> for bool {
    fn read(read: &mut PacketRead<'_>) -> Result<Self, PacketReadableError> {
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

fn read_str_with_limit<'a>(read: &mut PacketRead<'a>, limit: i32) -> Result<&'a str, PacketReadableError> {
    let slice = read_bytes_with_limit(read, limit)?;
    std::str::from_utf8(slice).map_err(|err| PacketReadableError::Any(err.into()))
}

fn read_bytes_with_limit<'a>(read: &mut PacketRead<'a>, limit: i32) -> Result<&'a [u8], PacketReadableError> {
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
    fn read(read: &mut PacketRead<'a>) -> Result<Self, PacketReadableError> {
        read_str_with_limit(read, DEFAULT_LIMIT)
    }
}

impl PacketReadable<'_> for String {
    fn read(read: &mut PacketRead<'_>) -> Result<Self, PacketReadableError> {
        <&'_ str>::read(read).map(|str| str.into())
    }
}

impl<'a> PacketReadable<'a> for Cow<'a, str> {
    fn read(read: &mut PacketRead<'a>) -> Result<Self, PacketReadableError> {
        Ok(Cow::Borrowed(<&'a str>::read(read)?))
    }
}

impl PacketWritable for &str {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        let bytes = self.as_bytes();
        VarInt::write_variant(&(bytes.len() as i32), write)?;
        write.write_bytes(bytes)
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
    fn read_variant(read: &mut PacketRead<'a>) -> Result<Vec<V>, PacketReadableError> {
        Ok(T::read_variant(read)?.to_vec())
    }
}

impl<'a, V: 'a + Clone, T: PacketVariantReadable<'a, &'a [V]> + ProtocolArray> PacketVariantReadable<'a, Cow<'a, [V]>> for T {
    fn read_variant(read: &mut PacketRead<'a>) -> Result<Cow<'a, [V]>, PacketReadableError> {
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
    fn read_variant(read: &mut PacketRead<'a>) -> Result<&'a [u8], PacketReadableError> {
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
    fn read_variant(read: &mut PacketRead<'a>) -> Result<Vec<ValueInner>, PacketReadableError> {
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
    fn read_variant(read: &mut PacketRead<'a>) -> Result<&'a [u8], PacketReadableError> {
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
    fn read_variant(read: &mut PacketRead<'a>) -> Result<Vec<ValueInner>, PacketReadableError> {
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
    fn read_variant(read: &mut PacketRead<'a>) -> Result<T, PacketReadableError> {
        let slice = read_bytes_with_limit(read, DEFAULT_LIMIT)?;
        serde_json::from_slice(slice).map_err(|err| PacketReadableError::Any(err.into()))
    }
}

impl<T: serde::Serialize> PacketVariantWritable<T> for ProtocolJson {
    fn write_variant<W>(object: &T, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        LengthProvidedBytesSlice::<VarInt, i32>::write_variant(
            &serde_json::to_vec(object)?, write,
        )
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
        impl PacketReadable<'_> for $num {
            fn read(read: &mut PacketRead<'_>) -> Result<Self, PacketReadableError> {
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
        impl PacketVariantReadable<'_, $num> for $var_num {
            fn read_variant(read: &mut PacketRead<'_>) -> Result<$num, PacketReadableError> {
                let mut value: $num = 0;
                let mut position: u8 = 0;
                loop {
                    let byte = read.take_byte()?;
                    value |= ((byte & 0x7F) << position) as $num;
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