use quote::quote_spanned;
use syn::{DeriveInput, parse_macro_input};

mod write;
mod util;

#[proc_macro_derive(PacketWritable, attributes(variant, order))]
pub fn packet_writable(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match write::write_impl(&parse_macro_input!(args as DeriveInput)) {
        Ok(ts) => ts,
        Err(err) => err.into_compile_error(),
    }.into()
}