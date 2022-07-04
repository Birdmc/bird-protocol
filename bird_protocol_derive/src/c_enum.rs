use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{DataEnum, Fields, Variant};
use crate::attribute::EnumFieldAttributes;

pub fn is_c_enum(data: &DataEnum) -> bool {
    data.variants
        .iter()
        .all(|variant| match variant.fields {
            Fields::Unit => true,
            _ => false,
        })
}

pub trait EnumVariantVisitor {
    fn visit(&mut self, variant: &Variant, value: TokenStream) -> syn::Result<()>;
}

pub fn visit_enum_variants(visitor: &mut impl EnumVariantVisitor, enum_ident: &Ident, data: &DataEnum) -> syn::Result<()> {
    match is_c_enum(data) {
        true => {
            for variant in &data.variants {
                let ident = &variant.ident;
                visitor.visit(variant, quote!{ #enum_ident :: #ident})?
            }
        }
        false => {
            let mut start = quote! {0};
            let mut counter: i32 = 0;
            for variant in &data.variants {
                match variant.attrs
                    .iter()
                    .find(|attr| attr.path.is_ident("pef"))
                    .map(|attr| attr.parse_args::<EnumFieldAttributes>()) {
                    Some(attr) => match attr?.value {
                        Some(expr) => start = expr.to_token_stream(),
                        None => counter += 1,
                    },
                    None => counter += 1,
                };
                visitor.visit(variant, quote! {(#start) + #counter})?
            }
        }
    }
    Ok(())
}