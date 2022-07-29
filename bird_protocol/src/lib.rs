#![feature(const_trait_impl)]
#![feature(associated_type_bounds)]

extern crate anyhow;

pub mod packet;
pub mod packet_types;
#[cfg(feature = "euclid")]
pub mod packet_euclid;
#[cfg(feature = "tokio-bytes")]
pub mod packet_bytes;
#[cfg(feature = "fastnbt")]
pub mod packet_fastnbt;
#[cfg(test)]
mod tests;

pub use crate::packet::*;
pub use crate::packet_types::*;