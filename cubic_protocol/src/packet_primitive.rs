use cubic_chat::component::ComponentType;
use cubic_chat::identifier::Identifier;
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;
use crate::packet::{CustomError, InputPacketBytes, OutputPacketBytes, PacketReadable, PacketReadableResult, PacketWritable, PacketWritableResult};
use crate::types::{Angle, BlockPosition, LengthProvidedArray, ProtocolJson, RemainingBytesArray, VarInt, VarLong};

#[async_trait::async_trait]
impl PacketReadable for u8 {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        input.take_byte().await.map_err(|err| err.into())
    }
}

#[async_trait::async_trait]
impl PacketWritable for u8 {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        output.write_byte(self).await.map_err(|err| CustomError::Error(err).into())
    }
}

#[async_trait::async_trait]
impl PacketReadable for i8 {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        u8::read(input).await.map(|val| val as i8)
    }
}

#[async_trait::async_trait]
impl PacketWritable for i8 {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        (self as u8).write(output).await
    }
}

#[async_trait::async_trait]
impl PacketReadable for bool {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        u8::read(input).await.map(|val| val != 0)
    }
}

#[async_trait::async_trait]
impl PacketWritable for bool {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        match self {
            true => 1_u8,
            false => 0_u8
        }.write(output).await
    }
}

#[async_trait::async_trait]
impl PacketReadable for Angle {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        let angle_value = u8::read(input).await? as f32;
        Ok(Angle::radians(angle_value * std::f32::consts::PI / 256.0))
    }
}

#[async_trait::async_trait]
impl PacketWritable for Angle {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        ((self.radians * 256.0 / std::f32::consts::PI) as u8).write(output).await
    }
}

const STRING_LIMIT: i32 = 32767;
const CHAT_LIMIT: i32 = 262144;

async fn read_string_with_limit(input: &mut impl InputPacketBytes, limit: i32) -> PacketReadableResult<String> {
    let length = i32::from(VarInt::read(input).await?);
    match length > limit {
        true => Err(CustomError::StaticStr("String is too big").into()),
        false => {
            let mut bytes = Vec::with_capacity(length as usize);
            unsafe { bytes.set_len(length as usize); }
            input.take_slice(&mut bytes).await?;
            Ok(std::str::from_utf8(bytes.as_slice())
                .map_err(|err| CustomError::Error(Box::new(err)))?
                .into()
            )
        }
    }
}

async fn write_string_with_limit(output: &mut impl OutputPacketBytes, str: &str, limit: i32) -> PacketWritableResult {
    let str_len = str.len() as i32;
    match str_len > limit {
        true => Err(CustomError::StaticStr("String is too big").into()),
        false => output.write_bytes(str.as_bytes()).await
            .map_err(|err| CustomError::Error(err).into())
    }
}

#[async_trait::async_trait]
impl PacketReadable for String {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        read_string_with_limit(input, STRING_LIMIT).await
    }
}

#[async_trait::async_trait]
impl PacketWritable for String {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        self.as_str().write(output).await
    }
}

#[async_trait::async_trait]
impl PacketWritable for &str {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        write_string_with_limit(output, self, STRING_LIMIT).await
    }
}

#[async_trait::async_trait]
impl PacketReadable for BlockPosition {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        let value = u64::read(input).await?;
        let mut x = (value >> 38) as i32;
        let mut y = (value & 0xFFF) as i32;
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
        Ok(Self::new(x, y, z))
    }
}

#[async_trait::async_trait]
impl PacketWritable for BlockPosition {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        let x = self.x as i64;
        let y = self.y as i64;
        let z = self.z as i64;
        (((x & 0x3FFFFFF) << 38) |
            ((z & 0x3FFFFFF) << 12) |
            (y & 0xFFF)
        ).write(output).await
    }
}

#[async_trait::async_trait]
impl PacketReadable for Identifier {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        Ok(Identifier::from_full(String::read(input).await?)
            .map_err(|err| CustomError::Error(Box::new(err)))?
        )
    }
}

#[async_trait::async_trait]
impl PacketWritable for Identifier {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        self.to_string().write(output).await
    }
}

#[async_trait::async_trait]
impl PacketReadable for ComponentType {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        let str = read_string_with_limit(input, CHAT_LIMIT).await?;
        serde_json::from_str(str.as_str())
            .map_err(|err| CustomError::Error(Box::new(err)).into())
    }
}

#[async_trait::async_trait]
impl PacketWritable for ComponentType {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        let str = serde_json::to_string(&self)
            .map_err(|err| CustomError::Error(Box::new(err)))?;
        write_string_with_limit(output, &str, CHAT_LIMIT).await
    }
}

#[async_trait::async_trait]
impl<T: DeserializeOwned> PacketReadable for ProtocolJson<T> {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        serde_json::from_str(String::read(input).await?.as_str())
            .map_err(|err| CustomError::Error(Box::new(err)).into())
            .map(|val| ProtocolJson::new(val))
    }
}

#[async_trait::async_trait]
impl<T: Serialize + Send + Sync> PacketWritable for ProtocolJson<T> {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        let str = serde_json::to_string(self.get())
            .map_err(|err| CustomError::Error(Box::new(err)))?;
        str.write(output).await
    }
}

#[async_trait::async_trait]
impl PacketReadable for Uuid {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        let mut bytes = [0_u8; std::mem::size_of::<u128>()];
        input.take_slice(&mut bytes).await?;
        Uuid::from_slice(&bytes)
            .map_err(|err| CustomError::Error(Box::new(err)).into())
    }
}

#[async_trait::async_trait]
impl PacketWritable for Uuid {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        output.write_bytes(self.as_bytes()).await
            .map_err(|err| CustomError::Error(err).into())
    }
}

#[async_trait::async_trait]
impl<T: PacketReadable> PacketReadable for Option<T> {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        let present = bool::read(input).await?;
        match present {
            true => Ok(Some(T::read(input).await?)),
            false => Ok(None),
        }
    }
}

#[async_trait::async_trait]
impl<T: PacketWritable + Send + Sync> PacketWritable for Option<T> {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        match self {
            Some(val) => {
                true.write(output).await?;
                val.write(output).await
            }
            None => false.write(output).await
        }
    }
}

pub async fn read_vec<T: PacketReadable>(
    length: usize, input: &mut impl InputPacketBytes) -> PacketReadableResult<Vec<T>> {
    let mut result = Vec::with_capacity(length);
    for _ in 0..length {
        result.push(T::read(input).await?);
    }
    Ok(result)
}

pub async fn write_vec<T: PacketWritable>(
    vec: Vec<T>, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
    for value in vec {
        value.write(output).await?
    }
    Ok(())
}

#[async_trait::async_trait]
impl<T: PacketReadable + Send + Sync> PacketReadable for RemainingBytesArray<T> {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        let length = input.remaining_bytes();
        Ok(RemainingBytesArray::new(read_vec(length, input).await?))
    }
}

#[async_trait::async_trait]
impl<T: PacketWritable + Send + Sync> PacketWritable for RemainingBytesArray<T> {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        write_vec(self.value, output).await?;
        Ok(())
    }
}

pub trait USizePossible {
    fn into_usize(self) -> usize;

    fn from_usize(value: usize) -> Self;
}

#[async_trait::async_trait]
impl<T: PacketReadable + Send + Sync, S: PacketReadable + USizePossible> PacketReadable for LengthProvidedArray<T, S> {
    async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
        let length = S::read(input).await?.into_usize();
        Ok(LengthProvidedArray::new(read_vec(length, input).await?))
    }
}

#[async_trait::async_trait]
impl<T: PacketWritable + Send + Sync,
    S: PacketWritable + USizePossible + Send + Sync> PacketWritable for LengthProvidedArray<T, S> {
    async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
        S::from_usize(self.value.len()).write(output).await?;
        write_vec(self.value, output).await
    }
}

macro_rules! num {
    ($type: ty) => {
        impl USizePossible for $type {
            fn into_usize(self) -> usize {
                self as usize
            }

            fn from_usize(value: usize) -> Self {
                value as Self
            }
        }

        #[async_trait::async_trait]
        impl PacketReadable for $type {
            async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
                let mut bytes = [0_u8; std::mem::size_of::<Self>()];
                input.take_slice(&mut bytes).await?;
                Ok(Self::from_le_bytes(bytes))
            }
        }

        #[async_trait::async_trait]
        impl PacketWritable for $type {
            async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
                output.write_bytes(&self.to_le_bytes()).await
                    .map_err(|err| CustomError::Error(err).into())
            }
        }
    };
    ($($type: ty$(,)*)*) => {
        $(num!($type);)*
    }
}

macro_rules! var_num {
    ($var_num_type: ty, $num_type: ty, $unsigned_num_type: ty) => {
        impl USizePossible for $var_num_type {
            fn into_usize(self) -> usize {
                <$num_type>::from(self) as usize
            }

            fn from_usize(value: usize) -> Self {
                <$var_num_type>::from(value as $num_type)
            }
        }

        #[async_trait::async_trait]
        impl PacketReadable for $var_num_type {
            async fn read(input: &mut impl InputPacketBytes) -> PacketReadableResult<Self> {
                const BITS: u8 = (std::mem::size_of::<$num_type>() * 8) as u8;
                let mut result: $num_type = 0;
                let mut position: u8 = 0;
                loop {
                    let current_byte = input.take_byte().await?;
                    result |= ((current_byte & 0x7F) as $num_type) << position;
                    if ((current_byte & 0x80) == 0) {
                        break;
                    }
                    position += 7;
                    if (position >= BITS) {
                        return Err(CustomError::StaticStr("Var num is too big").into());
                    }
                }
                Ok(<$var_num_type>::from(result))
            }
        }

        #[async_trait::async_trait]
        impl PacketWritable for $var_num_type {
            async fn write(self, output: &mut impl OutputPacketBytes) -> PacketWritableResult {
                let mut u_self = <$num_type>::from(self) as $unsigned_num_type;
                loop {
                    if ((u_self & !0x7F) == 0) {
                        (u_self as u8).write(output).await?;
                        break;
                    }
                    (((u_self as u8) & 0x7F) | 0x80).write(output).await?;
                    u_self >>= 7;
                }
                Ok(())
            }
        }
    }
}

num!(u16 i16 u32 i32 u64 i64 u128 i128 f32 f64);
var_num!(VarInt, i32, u32);
var_num!(VarLong, i64, u64);