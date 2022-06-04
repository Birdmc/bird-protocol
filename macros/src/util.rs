use std::collections::HashMap;
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use proc_macro_error::abort;
use quote::{ToTokens, quote};
use syn::{Field, Fields, Generics, parse_quote};
use crate::attr::ProtocolStruct;

pub fn iterate_fields(fields: &Fields, mut func: impl FnMut(String, &Field)) {
    match fields {
        Fields::Named(ref fields) => fields
            .named
            .iter()
            .for_each(|field| func(field.ident.as_ref().unwrap().to_string(), field)),
        Fields::Unnamed(ref fields) => {
            let mut counter = -1;
            fields.unnamed
                .iter()
                .for_each(|field| {
                    counter += 1;
                    func(counter.to_string(), field)
                })
        },
        Fields::Unit => {},
    }
}

pub fn collect_types(fields: &Fields, protocol_struct: &ProtocolStruct) -> Vec<(String, TokenStream)> {
    let mut types = Vec::new();
    let mut specific_order_types = HashMap::new();
    iterate_fields(
        &fields,
        |name, field| {
            let field_attributes = protocol_struct.fields.get(&name).unwrap();
            let field_type = field_attributes.variant.as_ref()
                .map(|(name, span)|
                    syn::Ident::new(name.as_str(), span.clone()).to_token_stream())
                .unwrap_or_else(|| field.ty.to_token_stream());
            let it_result = (name, field_type);
            match field_attributes.order {
                Some((order, span)) => {
                    specific_order_types
                        .insert(order, it_result)
                        .map(|_| abort!(span, "Order is repeated"));
                },
                None => types.push(it_result),
            }
        },
    );
    specific_order_types
        .into_iter()
        .for_each(|(index, val)| types.insert(index as usize, val));
    types
}

pub fn add_trait_to_generics(mut generics: Generics, trait_ts: TokenStream) -> Generics {
    for param in &mut generics.params {
        if let syn::GenericParam::Type(ref mut type_param) = param {
            type_param.bounds.push(parse_quote!(trait_ts));
        }
    }
    generics
}

pub fn get_crate() -> TokenStream {
    let found_crate = crate_name("cubic-protocol")
        .expect("cubic-protocol is present in `Cargo.toml`");

    match found_crate {
        FoundCrate::Itself => quote!{crate},
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!{#ident}
        }
    }
}

pub fn default_use() -> TokenStream {
    let cp_crate = get_crate();
    quote! {
        use #cp_crate::packet::*;
        use #cp_crate::packet_default::*;
        use #cp_crate::packet_primitive::*;
        use #cp_crate::types::*;
    }
}