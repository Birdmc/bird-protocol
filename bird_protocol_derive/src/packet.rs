/// Packet macro includes implementation of three traits:
/// - PacketWritable (Where PacketID goes first then other variables)
/// - PacketReadable (Without PacketID)
/// - Packet (ID, State, Side, Protocol)
/// Also it implement const Packet trait

use syn::DeriveInput;
use proc_macro2::{Ident, Span};
use quote::quote;
use crate::attribute::PacketAttributes;
use crate::readable_macro_impl;
use crate::util::get_crate;
use crate::writable::{build_writable_function_body, writable_trait_from_body};

pub fn validate_packet_attributes(packet_attributes: PacketAttributes) -> syn::Result<(i32, Ident, Ident, i32, bool, bool)> {
    let protocol = packet_attributes.protocol.unwrap().0;
    let id = packet_attributes.id.unwrap();
    let state = packet_attributes.state.unwrap();
    let side = packet_attributes.side.unwrap();
    let readable = packet_attributes.readable.map(|(value, _)| value).unwrap_or(true);
    let writable = packet_attributes.writable.map(|(value, _)| value).unwrap_or(true);

    if id.0 < 0 {
        return Err(syn::Error::new(id.1, "Id should not be negative"));
    }

    match state.0.as_str() {
        "Handshake" | "Login" | "Status" | "Play" => {}
        _ => return Err(syn::Error::new(state.1, "Possible states: Handshake, Login, Status, Play"))
    }

    match side.0.as_str() {
        "Client" | "Server" => {}
        _ => return Err(syn::Error::new(side.1, "Possible sides: Client, Server"))
    }

    let side = Ident::new(side.0.as_str(), side.1);
    let state = Ident::new(state.0.as_str(), state.1);

    Ok((id.0, side, state, protocol, readable, writable))
}

pub fn packet_macro_impl(input: &DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    let cp_crate = get_crate();
    let packet_attributes = input.attrs
        .iter()
        .find(|attribute| attribute.path.is_ident("packet"))
        .map(|attribute| attribute.parse_args::<PacketAttributes>())
        .unwrap_or_else(|| Err(syn::Error::new(Span::call_site(), "Packet attribute necessary")))?;

    let (id, side, state, protocol, readable, writable) =
        validate_packet_attributes(packet_attributes)?;

    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();
    let ident = &input.ident;

    // As because rust is not supporting const traits (Experimental)
    // So we create const variables to use ids in patterns and const functions
    let const_packet = proc_macro::TokenStream::from(quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            pub const ID: i32 = #id;
            pub const PROTOCOL: i32 = #protocol;

            pub const fn id() -> i32 {
                Self::ID
            }

            pub const fn protocol() -> i32 {
                Self::PROTOCOL
            }

            pub const fn side() -> #cp_crate::packet::PacketSide {
                #cp_crate::packet::PacketSide::#side
            }

            pub const fn state() -> #cp_crate::packet::PacketState {
                #cp_crate::packet::PacketState::#state
            }
        }
    });

    let packet_trait = proc_macro::TokenStream::from(quote! {
        impl #impl_generics #cp_crate::packet::Packet for #ident #ty_generics #where_clause {
            fn id() -> i32 {
                Self::id()
            }

            fn side() -> #cp_crate::packet::PacketSide {
                Self::side()
            }

            fn state() -> #cp_crate::packet::PacketState {
                Self::state()
            }

            fn protocol() -> i32 {
                Self::protocol()
            }
        }
    });

    let packet_writable_trait = match writable {
        true => {
            let write_body = build_writable_function_body(input)?;
            let write_body = quote! {
                <#cp_crate::types::VarInt as #cp_crate::packet::PacketWritable>::write(
                    &<Self as #cp_crate::packet::Packet>::id().into(),
                    output
                ).await?;
                #write_body
            };
            writable_trait_from_body(input, write_body)?
        },
        false => proc_macro::TokenStream::new()
    };

    let packet_readable_trait = match readable {
        true => readable_macro_impl(input)?,
        false => proc_macro::TokenStream::new()
    };

    Ok(proc_macro::TokenStream::from_iter(
        vec![const_packet, packet_trait, packet_writable_trait, packet_readable_trait]
    ))
}