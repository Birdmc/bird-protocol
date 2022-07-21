use std::borrow::Cow;
use std::marker::PhantomData;
use anyhow::Error;
use crate::packet::{PacketRead, PacketReadable, PacketReadableError, PacketWritable, PacketVariantReadable, PacketVariantWritable, PacketWrite};

pub struct VarInt;

pub struct VarLong;

pub struct RemainingSlice;

pub struct RemainingBytesSlice;

pub struct RemainingRawSlice;

pub struct LengthProvidedSlice<L: PacketLength>(PhantomData<L>);

pub struct LengthProvidedBytesSlice<L: PacketLength>(PhantomData<L>);

pub struct LengthProvidedRawSlice<L: PacketLength>(PhantomData<L>);

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
    let length = VarInt::read_variant(read)?;
    match length > limit {
        true => Err(PacketReadableError::Any(anyhow::Error::msg("Too big string"))),
        false => {
            let slice = read.take_slice(length as usize)?;
            std::str::from_utf8(slice).map_err(|err| PacketReadableError::Any(err.into()))
        }
    }
}

const STRING_LIMIT: i32 = 32767;
const CHAT_LIMIT: i32 = 262144;

impl<'a> PacketReadable<'a> for &'a str {
    fn read(read: &mut PacketRead<'a>) -> Result<Self, PacketReadableError> {
        read_str_with_limit(read, STRING_LIMIT)
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
        write.write_bytes(self.as_bytes())
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

impl<'a, V: 'a + Clone, T: PacketVariantReadable<'a, &'a [V]>> PacketVariantReadable<'a, Vec<V>> for T {
    fn read_variant(read: &mut PacketRead<'a>) -> Result<Vec<V>, PacketReadableError> {
        Ok(T::read_variant(read)?.to_vec())
    }
}

impl<'a, V: 'a + Clone, T: PacketVariantReadable<'a, &'a [V]> + Sized> PacketVariantReadable<'a, Cow<'a, [V]>> for T {
    fn read_variant(read: &mut PacketRead<'a>) -> Result<Cow<'a, [V]>, PacketReadableError> {
        Ok(Cow::Borrowed(T::read_variant(read)?))
    }
}

impl<'a, V: 'a + Clone, T: PacketVariantWritable<&'a [V]> + PacketVariantWritable<Vec<V>>> PacketVariantWritable<Cow<'a, [V]>> for T {
    fn write_variant<W>(object: &Cow<'a, [V]>, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        match object {
            Cow::Owned(ref owned) => T::write_variant(owned, write),
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

impl<'a, T: PacketReadable<'a>> PacketVariantReadable<'a, Vec<T>> for RemainingSlice {
    fn read_variant(read: &mut PacketRead<'a>) -> Result<Vec<T>, PacketReadableError> {
        let mut result = Vec::new();
        while read.available() != 0 {
            result.push(T::read(read)?);
        }
        Ok(result)
    }
}

impl<T: PacketWritable> PacketVariantWritable<&[T]> for RemainingSlice {
    fn write_variant<W>(object: &&[T], write: &mut W) -> Result<(), Error> where W: PacketWrite {
        for element in *object {
            element.write(write)?
        }
        Ok(())
    }
}

impl<T: PacketWritable> PacketVariantWritable<Vec<T>> for RemainingSlice {
    fn write_variant<W>(object: &Vec<T>, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        Self::write_variant(&object.as_slice(), write)
    }
}

// TODO think to move it to the unsafe function, because it is actually unsafe or implement it for specific types to make it safe
// Unsafe: if T type contains pointer types (Functions, Pointers, References, etc)
impl<'a, T: Copy + Sized> PacketVariantReadable<'a, &'a [T]> for RemainingRawSlice {
    fn read_variant(read: &mut PacketRead<'a>) -> Result<&'a [T], PacketReadableError> {
        let remaining_slice: &[u8] = RemainingBytesSlice::read_variant(read)?;
        match remaining_slice.len() % std::mem::size_of::<T>() == 0 {
            true => unsafe {
                Ok(std::slice::from_raw_parts(
                    remaining_slice.as_ptr() as *const T, remaining_slice.len(),
                ))
            },
            false => Err(PacketReadableError::Any(Error::msg("Remaining slice length is not match")))
        }
    }
}

impl<T: Copy + Sized> PacketVariantWritable<&[T]> for RemainingRawSlice {
    fn write_variant<W>(object: &&[T], write: &mut W) -> Result<(), Error> where W: PacketWrite {
        write.write_bytes(unsafe {
            std::slice::from_raw_parts(
                object.as_ptr() as *const u8,
                std::mem::size_of::<T>() * object.len(),
            )
        })
    }
}

impl<T: Copy + Sized> PacketVariantWritable<Vec<T>> for RemainingRawSlice {
    fn write_variant<W>(object: &Vec<T>, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        Self::write_variant(&object.as_slice(), write)
    }
}

pub trait PacketLength {
    fn into_length(self) -> usize;

    fn from_length(length: usize) -> Self;
}

impl<'a, L: PacketLength + PacketReadable<'a>> PacketVariantReadable<'a, &'a [u8]> for LengthProvidedBytesSlice<L> {
    fn read_variant(read: &mut PacketRead<'a>) -> Result<&'a [u8], PacketReadableError> {
        let length = L::read(read)?.into_length();
        read.take_slice(length)
    }
}

impl<L: PacketLength + PacketWritable> PacketVariantWritable<&[u8]> for LengthProvidedBytesSlice<L> {
    fn write_variant<W>(object: &&[u8], write: &mut W) -> Result<(), Error> where W: PacketWrite {
        L::from_length(object.len()).write(write)?;
        write.write_bytes(object)
    }
}

impl<L: PacketLength + PacketWritable> PacketVariantWritable<Vec<u8>> for LengthProvidedBytesSlice<L> {
    fn write_variant<W>(object: &Vec<u8>, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        Self::write_variant(&object.as_slice(), write)
    }
}

impl<'a, L: PacketLength + PacketReadable<'a>, T: PacketReadable<'a>>
PacketVariantReadable<'a, Vec<T>> for LengthProvidedSlice<L> {
    fn read_variant(read: &mut PacketRead<'a>) -> Result<Vec<T>, PacketReadableError> {
        let length = L::read(read)?.into_length();
        let mut result = Vec::with_capacity(length);
        for _ in 0..length {
            result.push(T::read(read)?);
        }
        Ok(result)
    }
}

impl<L: PacketLength + PacketWritable, T: PacketWritable>
PacketVariantWritable<&[T]> for LengthProvidedSlice<L> {
    fn write_variant<W>(object: &&[T], write: &mut W) -> Result<(), Error> where W: PacketWrite {
        L::from_length(object.len()).write(write)?;
        for element in *object {
            element.write(write)?
        }
        Ok(())
    }
}

impl<L: PacketLength + PacketWritable, T: PacketWritable>
PacketVariantWritable<Vec<T>> for LengthProvidedSlice<L> {
    fn write_variant<W>(object: &Vec<T>, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        Self::write_variant(&object.as_slice(), write)
    }
}

// TODO same as RemainingRawSlice
impl<'a, L: PacketLength + PacketReadable<'a>, T: Clone + Sized>
PacketVariantReadable<'a, &'a [T]> for LengthProvidedRawSlice<L> {
    fn read_variant(read: &mut PacketRead<'a>) -> Result<&'a [T], PacketReadableError> {
        let length = L::read(read)?.into_length();
        // TODO should we check for panic situations?
        match usize::MAX / std::mem::size_of::<T>() < length {
            true => Err(PacketReadableError::Any(Error::msg("Too big slice"))),
            false => {
                let length = length * std::mem::size_of::<T>();
                let slice = read.take_slice(length)?;
                Ok(unsafe {
                    std::slice::from_raw_parts(slice.as_ptr() as *const T, length)
                })
            }
        }
    }
}

impl<L: PacketLength + PacketWritable, T: Clone + Sized>
PacketVariantWritable<&[T]> for LengthProvidedRawSlice<L> {
    fn write_variant<W>(object: &&[T], write: &mut W) -> Result<(), Error> where W: PacketWrite {
        L::from_length(object.len()).write(write)?;
        write.write_bytes(unsafe {
            std::slice::from_raw_parts(
                object.as_ptr() as *const u8,
                object.len() * std::mem::size_of::<T>(),
            )
        })
    }
}

impl<L: PacketLength + PacketWritable, T: Clone + Sized>
PacketVariantWritable<Vec<T>> for LengthProvidedRawSlice<L> {
    fn write_variant<W>(object: &Vec<T>, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        Self::write_variant(&object.as_slice(), write)
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

length_impl!(u8 i8 u16 i16 u32 i32 u64 i64);

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
                    if ((byte & 0x80) == 0) {
                        break Ok(value)
                    }
                    position += 7;
                    if (position >= (std::mem::size_of::<$num>() * 8) as u8) {
                        break Err(PacketReadableError::Any(anyhow::Error::msg("Var number is too long")))
                    }
                }
            }
        }

        impl PacketVariantWritable<$num> for $var_num {
            fn write_variant<W>(object: & $num, write: &mut W) -> Result<(), Error> where W: PacketWrite {
                let mut value: $unsigned_num = *object as $unsigned_num;
                loop {
                    if ((value & !0x7F) == 0) {
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

number_impl!(u16 i16 u32 i32 u64 i64 u128 i128);
var_number_impl!(VarInt, i32, u32);