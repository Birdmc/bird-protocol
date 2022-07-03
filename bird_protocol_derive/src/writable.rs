use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, Field, Fields, Data, parse_quote};
use syn::spanned::Spanned;
use crate::attribute::{EnumAttributes, FieldAttributes};
use crate::c_enum::is_c_enum;
use crate::fields::{EnumFields, FieldVisitor, visit_fields};
use crate::util::{add_trait_bounds, async_trait, get_crate};

pub struct WritableVisitor {
    object_ts: TokenStream,
    row_order: Vec<TokenStream>,
}

impl WritableVisitor {
    pub fn new(object_ts: TokenStream) -> WritableVisitor {
        WritableVisitor {
            object_ts,
            row_order: Vec::new(),
        }
    }

    pub fn get_result(self) -> TokenStream {
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
        let writable_value = match attributes.write.or(attributes.variant) {
            Some(variant) => {
                (
                    quote! {#variant},
                    quote! {<#variant>::from(& #object_ts #name)},
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
        self.row_order.push(writable);
        Ok(())
    }
}

pub fn write_ts(ty: TokenStream, value: TokenStream) -> TokenStream {
    let cp_crate = get_crate();
    quote! {<#ty as #cp_crate::packet::PacketWritable>::write(& #value, output).await?;}
}

#[allow(unused)]
pub fn generate_write(object_ts: TokenStream, fields: &Fields) -> syn::Result<TokenStream> {
    let mut visitor = WritableVisitor::new(object_ts);
    visit_fields(fields, &mut visitor)?;
    Ok(visitor.get_result())
}

pub fn build_writable_function_body(input: &DeriveInput) -> syn::Result<TokenStream> {
    Ok(match input.data {
        Data::Struct(ref data_struct) => {
            let mut visitor = WritableVisitor::new(quote! {self.});
            visit_fields(&data_struct.fields, &mut visitor)?;
            visitor.get_result_with_return()
        }
        Data::Enum(ref data_enum) => match is_c_enum(data_enum) {
            true => {
                let enum_attributes = match EnumAttributes::find_one(&input.attrs)? {
                    Some(attrs) => attrs.into_filled()?,
                    None => return Err(syn::Error::new(
                        Span::call_site(), "not C-like enums is not supported"))
                };
                let cp_crate = get_crate();
                let primitive = enum_attributes.primitive.unwrap();
                let variant = enum_attributes.variant.unwrap();
                quote! {
                    <#variant as #cp_crate::packet::PacketWritable>::write(
                        &#variant::from(*self as #primitive), output
                    ).await
                }
            }
            false => {
                let mut variants = TokenStream::new();
                for variant in &data_enum.variants {
                    let enum_fields = EnumFields::build(&variant.fields)?;
                    let mut writable_visitor = WritableVisitor::new(enum_fields.prefix());
                    visit_fields(&variant.fields, &mut writable_visitor)?;
                    let variant_ident = &variant.ident;
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
        }
        Data::Union(_) => return Err(
            syn::Error::new(input.span(), "Union type is not supported")
        )
    })
}

pub fn writable_trait_from_body(input: &DeriveInput, func_body: TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let ident = &input.ident;
    let cp_crate = get_crate();
    let generics = add_trait_bounds(
        input.generics.clone(),
        vec![parse_quote! {#cp_crate::packet::PacketWritable}],
    );
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(async_trait(quote! {
        impl #impl_generics #cp_crate::packet::PacketWritable for #ident #ty_generics #where_clause {
            async fn write(&self, output: &mut impl #cp_crate::packet::OutputPacketBytes) -> #cp_crate::packet::PacketWritableResult {
                #func_body
            }
        }
    }))
}

pub fn writable_macro_impl(input: &DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    let func_body = build_writable_function_body(&input)?;
    writable_trait_from_body(input, func_body)
}