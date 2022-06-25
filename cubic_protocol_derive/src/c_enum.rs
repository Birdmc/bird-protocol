use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{DataEnum, Fields};

pub fn is_c_enum(data: &DataEnum) -> bool {
    data.variants
        .iter()
        .all(|variant| match variant.fields {
            Fields::Unit => true,
            _ => false,
        })
}

pub trait CEnumFieldsVisitor {
    fn visit(&mut self, name: &Ident, value: TokenStream);
}

pub fn visit_c_enum(data: &DataEnum, visitor: &mut impl CEnumFieldsVisitor) -> syn::Result<bool> {
    Ok(match is_c_enum(data) {
        true => {
            let mut start = quote! {0};
            let mut counter = 0;
            for variant in &data.variants {
                match variant.discriminant {
                    Some((_, ref expr)) => {
                        start = quote! {#expr};
                        counter = 0
                    }
                    None => counter += 1
                }
                visitor.visit(&variant.ident, quote! {((#start) as i32) + #counter})
            }
            true
        }
        false => false,
    })
}
