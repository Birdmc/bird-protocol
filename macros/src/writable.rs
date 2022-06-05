use std::collections::HashMap;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{DeriveInput, Field, Fields, Data, parse_quote};
use syn::spanned::Spanned;
use crate::attribute::FieldAttributes;
use crate::fields::{EnumFields, FieldVisitor, visit_fields};
use crate::util::{add_trait_bounds, async_trait, get_crate};

pub struct WritableVisitor {
    object_ts: TokenStream,
    spec_order: HashMap<i32, TokenStream>,
    row_order: Vec<TokenStream>,
}

impl WritableVisitor {
    pub fn new(object_ts: TokenStream) -> WritableVisitor {
        WritableVisitor {
            object_ts,
            spec_order: HashMap::new(),
            row_order: Vec::new(),
        }
    }

    pub fn get_result(mut self) -> TokenStream {
        self.spec_order
            .into_iter()
            .for_each(|(key, ts)| self.row_order.insert(key as usize, ts));
        TokenStream::from_iter(self.row_order)
    }

    pub fn get_result_with_return(self) -> TokenStream {
        let result = self.get_result();
        quote! {
            #result
            Ok(())
        }
    }
}

impl FieldVisitor for WritableVisitor {
    fn visit(&mut self, name: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()> {
        let WritableVisitor { object_ts, .. } = self;
        let writable_value = match attributes.variant {
            Some((variant, span)) => {
                let variant_ident = Ident::new(variant.as_str(), span);
                (
                    quote! {#variant_ident},
                    quote! {#variant_ident::from(#object_ts #name)},
                )
            }
            None => {
                let field_ty = &field.ty;
                (
                    quote! {#field_ty},
                    quote! {#object_ts #name},
                )
            }
        };
        let writable = write_ts(writable_value.0, writable_value.1);
        match attributes.order {
            Some((order, span)) =>
                if let Some(_) = self.spec_order.insert(order, writable) {
                    return Err(syn::Error::new(span, "Order repeats"));
                },
            None => self.row_order.push(writable),
        }
        Ok(())
    }
}

pub fn write_ts(ty: TokenStream, value: TokenStream) -> TokenStream {
    let cp_crate = get_crate();
    quote! {<#ty as #cp_crate::packet::PacketWritable>::write(#value, output).await?;}
}

#[allow(unused)]
pub fn generate_write(object_ts: TokenStream, fields: &Fields) -> syn::Result<TokenStream> {
    let mut visitor = WritableVisitor::new(object_ts);
    visit_fields(fields, &mut visitor)?;
    Ok(visitor.get_result())
}

pub fn writable_macro_impl(input: DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    let func_body = match input.data {
        Data::Struct(data_struct) => {
            let mut visitor = WritableVisitor::new(quote! {self.});
            visit_fields(&data_struct.fields, &mut visitor)?;
            visitor.get_result_with_return()
        }
        Data::Enum(data_enum) => {
            let mut variants = TokenStream::new();
            for variant in data_enum.variants {
                let enum_fields = EnumFields::build(&variant.fields)?;
                let mut writable_visitor = WritableVisitor::new(enum_fields.prefix());
                visit_fields(&variant.fields, &mut writable_visitor)?;
                let variant_ident = variant.ident;
                let variant_arguments = enum_fields.arguments();
                let variant_body = writable_visitor.get_result_with_return();
                variants = quote! {
                    #variants
                    match Self::#variant_ident #variant_arguments => {
                        #variant_body
                    },
                }
            }
            match variants.is_empty() {
                true => quote! { Ok(()) },
                false => quote! {
                    match self {
                        #variants
                    }
                },
            }
        }
        Data::Union(_) => return Err(
            syn::Error::new(input.span(), "Union type is not supported")
        )
    };
    let ident = input.ident;
    let cp_crate = get_crate();
    let generics = add_trait_bounds(
        input.generics,
        vec![parse_quote! {#cp_crate::packet::PacketWritable}],
    );
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(async_trait(quote! {
        impl #impl_generics #cp_crate::packet::PacketWritable for #ident #ty_generics #where_clause {
            async fn write(self, output: &mut impl #cp_crate::packet::OutputPacketBytes) -> #cp_crate::packet::PacketWritableResult {
                #func_body
            }
        }
    }))
}