use proc_macro2::{Span, TokenStream, Ident};
use quote::quote;
use syn::{Generics, TypeParamBound};

pub fn async_trait(implementation: TokenStream) -> proc_macro::TokenStream {
    (quote! {
        #[async_trait::async_trait]
        #implementation
    }).into()
}

pub fn get_crate() -> TokenStream {
    match proc_macro_crate::crate_name("cubic-protocol").unwrap() {
        proc_macro_crate::FoundCrate::Itself => quote! {crate},
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = Ident::new(name.as_str(), Span::call_site());
            quote!{#ident}
        }
    }
}

pub fn add_trait_bounds(mut generics: Generics, traits: Vec<TypeParamBound>) -> Generics {
    for param in &mut generics.params {
        if let syn::GenericParam::Type(ref mut type_param) = param {
            traits
                .iter()
                .for_each(|bound| type_param.bounds.push(bound.clone()));
        }
    }
    generics
}