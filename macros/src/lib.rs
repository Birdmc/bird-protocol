mod attribute;
mod fields;
mod writable;
mod util;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};
use crate::writable::writable_macro_impl;

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(Packet, attributes(packet, pf))]
pub fn packet(_body: TokenStream) -> TokenStream {
    TokenStream::new()
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(PacketWritable, attributes(pf))]
pub fn packet_writable(body: TokenStream) -> TokenStream {
    writable_macro_impl(parse_macro_input!(body as DeriveInput))
        .map_err(|err| proc_macro_error::abort!(err.span(), "{}", err))
        .unwrap()
}

#[proc_macro_error::proc_macro_error]
#[proc_macro_derive(PacketReadable, attributes(pf))]
pub fn packet_readable(_body: TokenStream) -> TokenStream {
    TokenStream::new()
}