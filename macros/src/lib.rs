mod attribute;
mod fields;
mod writable;
mod readable;
mod packet;
mod util;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};
use crate::packet::packet_macro_impl;
use crate::readable::readable_macro_impl;
use crate::writable::writable_macro_impl;

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(Packet, attributes(packet, pf))]
pub fn packet(body: TokenStream) -> TokenStream {
    packet_macro_impl(&parse_macro_input!(body as DeriveInput))
        .map_err(|err| proc_macro_error::abort!(err.span(), "{}", err))
        .unwrap()
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(PacketWritable, attributes(pf))]
pub fn packet_writable(body: TokenStream) -> TokenStream {
    writable_macro_impl(&parse_macro_input!(body as DeriveInput))
        .map_err(|err| proc_macro_error::abort!(err.span(), "{}", err))
        .unwrap()
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(PacketReadable, attributes(pf))]
pub fn packet_readable(body: TokenStream) -> TokenStream {
    readable_macro_impl(&parse_macro_input!(body as DeriveInput))
        .map_err(|err| proc_macro_error::abort!(err.span(), "{}", err))
        .unwrap()
}
