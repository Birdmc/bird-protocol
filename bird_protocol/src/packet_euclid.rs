use anyhow::Error;
use euclid::default::Vector3D;
use crate::packet::{PacketRead, PacketReadable, PacketReadableError, PacketVariantReadable, PacketVariantWritable, PacketWritable, PacketWrite};
use crate::packet_types::{Angle, BlockPosition};

impl<'a> PacketReadable<'a> for euclid::Angle<f32> {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        Ok(euclid::Angle::radians(Angle::read_variant(read)?))
    }
}

impl<'a> PacketReadable<'a> for euclid::Angle<f64> {
    fn read<R>(read: &mut R) -> Result<Self, PacketReadableError> where R: PacketRead<'a> {
        euclid::Angle::<f32>::read(read).map(|angle| angle.cast())
    }
}

impl PacketWritable for euclid::Angle<f32> {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        Angle::write_variant(&self.radians, write)
    }
}

impl PacketWritable for euclid::Angle<f64> {
    fn write<W>(&self, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        Angle::write_variant(&(self.radians as f32), write)
    }
}

impl<'a> PacketVariantReadable<'a, euclid::default::Vector3D<i32>> for BlockPosition {
    fn read_variant<R>(read: &mut R) -> Result<euclid::default::Vector3D<i32>, PacketReadableError>
        where R: PacketRead<'a> {
        let position = BlockPosition::read(read)?;
        Ok(euclid::Vector3D::new(position.x, position.y as i32, position.z))
    }
}

impl PacketVariantWritable<euclid::default::Vector3D<i32>> for BlockPosition {
    fn write_variant<W>(object: &Vector3D<i32>, write: &mut W) -> Result<(), Error> where W: PacketWrite {
        BlockPosition {
            x: object.x,
            y: object.y as i16,
            z: object.z,
        }.write(write)
    }
}

macro_rules! angle_variant_impl {
    ($inner: ty) => {
        impl<'a> PacketVariantReadable<'a, euclid::Angle<$inner>> for Angle {
            fn read_variant<R>(read: &mut R) -> Result<euclid::Angle<$inner>, PacketReadableError>
                where R: PacketRead<'a> {
                euclid::Angle::read(read)
            }
        }

        impl PacketVariantWritable<euclid::Angle<$inner>> for Angle {
            fn write_variant<W>(object: &euclid::Angle<$inner>, write: &mut W) -> Result<(), anyhow::Error>
                where W: PacketWrite {
                object.write(write)
            }
        }
    };
    ($($inner:ty$(,)*)*) => {
        $(angle_variant_impl!($inner);)*
    }
}

angle_variant_impl!(f32, f64);