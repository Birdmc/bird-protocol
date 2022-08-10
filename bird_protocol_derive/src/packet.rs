use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::DeriveInput;
use crate::util::{get_bird_protocol_crate, PacketAttributes};

pub fn packet_impl(args: &DeriveInput) -> syn::Result<TokenStream> {
    let PacketAttributes { bound, state, id } =
        match args.attrs.iter().find(|attr| attr.path.is_ident("packet")) {
            Some(attr) => attr.parse_args()?,
            None => return Err(syn::Error::new(Span::call_site(), "packet attribute is not found"))
        };
    let DeriveInput { ident, generics, .. } = args;
    let protocol_crate = get_bird_protocol_crate();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #ident #ty_generics #where_clause {
            pub const ID: i32 = ( #id ) as i32;
            pub const BOUND: #protocol_crate ::packet::PacketBound = #protocol_crate ::packet::PacketBound:: #bound;
            pub const STATE: #protocol_crate ::packet::PacketState = #protocol_crate ::packet::PacketState:: #state;
        }

        impl #impl_generics #protocol_crate ::packet::Packet for #ident #ty_generics #where_clause {
            fn bound() -> #protocol_crate ::packet::PacketBound {
                Self::BOUND
            }

            fn state() -> #protocol_crate ::packet::PacketState {
                Self::STATE
            }

            fn id() -> i32 {
                Self::ID
            }
        }
    })
}