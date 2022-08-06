use syn::{DeriveInput, parse_macro_input};

mod write;
mod util;
mod read;

#[proc_macro_derive(PacketWritable, attributes(variant, var, order, lifetime, enum_type, enum_variant, value))]
pub fn packet_writable(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match write::write_impl(&parse_macro_input!(args as DeriveInput)) {
        Ok(ts) => ts,
        Err(err) => err.into_compile_error(),
    }.into()
}

#[proc_macro_derive(PacketReadable, attributes(variant, var, order, lifetime, enum_type, enum_variant, value))]
pub fn packet_readable(args: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match read::read_impl(&parse_macro_input!(args as DeriveInput)) {
        Ok(ts) => ts,
        Err(err) => err.into_compile_error(),
    }.into()
}