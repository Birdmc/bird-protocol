/// Packet macro includes implementation of three traits:
/// - PacketWritable (Where PacketID goes first then other variables)
/// - PacketReadable (Without PacketID)
/// - Packet (ID, State, Side, Protocol)
/// Also it implement const Packet trait

use syn::DeriveInput;
use proc_macro2::Span;
use quote::quote;
use crate::attribute::{expr_to_bool, PacketAttributes};
use crate::readable_macro_impl;
use crate::util::get_crate;
use crate::writable::{build_writable_function_body, writable_trait_from_body};

pub fn packet_macro_impl(input: &DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    let cp_crate = get_crate();
    let packet_attributes = input.attrs
        .iter()
        .find(|attribute| attribute.path.is_ident("packet"))
        .map(|attribute| attribute.parse_args::<PacketAttributes>())
        .unwrap_or_else(|| Err(syn::Error::new(Span::call_site(), "Packet attribute necessary")))?;

    let id = packet_attributes.id.unwrap();
    let protocol = packet_attributes.protocol.unwrap();
    let side = packet_attributes.side.unwrap();
    let state = packet_attributes.state.unwrap();
    let writable = match packet_attributes.writable {
        Some(ref expr) => expr_to_bool(expr)?,
        None => true,
    };
    let readable = match packet_attributes.readable {
        Some(ref expr) => expr_to_bool(expr)?,
        None => true,
    };

    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();
    let ident = &input.ident;

    // As because rust is not supporting const traits (Experimental)
    // So we create const variables to use ids in patterns and const functions
    // also consts used in packet_node macro
    let const_packet = proc_macro::TokenStream::from(quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            pub const ID: i32 = (#id) as i32;
            pub const PROTOCOL: i32 = (#protocol) as i32;

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