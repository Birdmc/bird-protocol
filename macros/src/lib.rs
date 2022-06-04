mod attr;
mod readable;
mod writable;
mod packet;
mod util;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};
use crate::attr::{ProtocolClass};
use proc_macro_error::proc_macro_error;
use crate::writable::writable_impl;

#[proc_macro_error]
#[proc_macro_derive(PacketReadable, attributes(packet, pf, packet_field))]
pub fn packet_readable(input: TokenStream) -> TokenStream {
    TokenStream::new()
}

#[proc_macro_error]
#[proc_macro_derive(PacketWritable, attributes(packet, pf, packet_field))]
pub fn packet_writable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let output = writable_impl(input);
    println!("{}", output);
    output.into()
}

#[proc_macro_error]
#[proc_macro_derive(Packet, attributes(packet, pf, packet_field))]
pub fn packet(input: TokenStream) -> TokenStream {
    TokenStream::new()
}