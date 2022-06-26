use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, Field, Fields, Data, parse_quote, DataEnum, Attribute};
use crate::attribute::{EnumAttributes, FieldAttributes};
use crate::c_enum::is_c_enum;
use crate::fields::{FieldVisitor, visit_fields};
use crate::util::{add_trait_bounds, async_trait, get_crate};

pub trait ReadVisitor {
    fn get_body(&self) -> TokenStream;
}

struct UnnamedReadVisitor {
    fields: TokenStream,
}

struct NamedReadVisitor {
    fields: TokenStream,
}

impl UnnamedReadVisitor {
    pub fn new() -> Self {
        UnnamedReadVisitor {
            fields: quote! {},
        }
    }
}

impl ReadVisitor for UnnamedReadVisitor {
    fn get_body(&self) -> TokenStream {
        let UnnamedReadVisitor { fields } = self;
        quote! {
            (#fields)
        }
    }
}

impl FieldVisitor for UnnamedReadVisitor {
    fn visit(&mut self, name: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()> {
        let UnnamedReadVisitor { fields } = self;
        let variable = read_ts_variant(&name, field, &attributes);
        self.fields = quote! {
            #fields
            #variable,
        };
        Ok(())
    }
}

impl NamedReadVisitor {
    pub fn new() -> Self {
        NamedReadVisitor {
            fields: quote! {}
        }
    }
}

impl ReadVisitor for NamedReadVisitor {
    fn get_body(&self) -> TokenStream {
        let NamedReadVisitor { fields } = self;
        quote! {
            {
                #fields
            }
        }
    }
}

impl FieldVisitor for NamedReadVisitor {
    fn visit(&mut self, name: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()> {
        let NamedReadVisitor { fields } = self;
        let variable = read_ts_variant(&name, &field, &attributes);
        self.fields = quote! {
            #fields
            #name: #variable,
        };
        Ok(())
    }
}

pub fn read_ts(ty: TokenStream) -> TokenStream {
    let cp_crate = get_crate();
    quote! { <#ty as #cp_crate::packet::PacketReadable>::read(input).await? }
}

fn read_ts_variant(_ident: &Ident, field: &Field, attributes: &FieldAttributes) -> TokenStream {
    match &attributes.variant {
        Some((variant, span)) => {
            let ident = Ident::new(variant.as_str(), span.clone());
            let read_st = read_ts(quote! { #ident });
            quote! { #read_st .into() }
        }
        None => {
            let field_ty = &field.ty;
            read_ts(quote! { #field_ty })
        }
    }
}

pub fn build_read_for(fields: &Fields) -> syn::Result<TokenStream> {
    Ok(match fields {
        Fields::Unit => quote! {},
        Fields::Named(_) => {
            let mut visitor = NamedReadVisitor::new();
            visit_fields(fields, &mut visitor)?;
            visitor.get_body()
        }
        Fields::Unnamed(_) => {
            let mut visitor = UnnamedReadVisitor::new();
            visit_fields(fields, &mut visitor)?;
            visitor.get_body()
        }
    })
}

pub fn build_read_for_enum(attrs: &Vec<Attribute>, enum_ident: &Ident, data_enum: &DataEnum) -> syn::Result<TokenStream> {
    match is_c_enum(data_enum) {
        true => {
            let enum_attributes =
                match EnumAttributes::find_one(&attrs)?
                {
                    Some(attr) => attr,
                    None => return Err(syn::Error::new(
                        Span::call_site(), "didn't found packet_enum attribute")),
                };
            let cp_crate = get_crate();
            let enum_attributes = enum_attributes.into_filled()?;
            let (value, span) = enum_attributes.primitive.unwrap();
            let primitive = Ident::new(value.as_str(), span);
            let (value, span) = enum_attributes.variant.unwrap();
            let variant = Ident::new(value.as_str(), span);
            let mut counter: usize = 0;
            let mut values = quote! {};
            let mut matches = quote! {};
            data_enum
                .variants
                .iter()
                .for_each(|variant| {
                    let variable_ident = Ident::new(
                        format!("__{}", counter).as_str(), Span::call_site());
                    let variant_ident = &variant.ident;
                    values = quote!{
                        #values
                        const #variable_ident: #primitive = #enum_ident::#variant_ident as #primitive;
                    };
                    matches = quote! {
                        #matches
                        #variable_ident => Ok(Self::#variant_ident),
                    };
                    counter += 1;
                });
            Ok(quote!{
                let value = #primitive::from(
                    <#variant as #cp_crate::packet::PacketReadable>::read(input).await?);
                #values
                match value {
                    #matches
                    _ => Err(#cp_crate::packet::PacketReadableError::Custom(
                        #cp_crate::packet::CustomError::StaticStr("Bad enum value")
                    ))
                }
            })
        }
        false => Err(syn::Error::new(Span::call_site(), "not C-like enums is not supported"))
    }
}

pub fn readable_macro_impl(input: &DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    let func_body = match input.data {
        Data::Struct(ref data_struct) => {
            let read = build_read_for(&data_struct.fields)?;
            quote! {
                Ok(Self #read)
            }
        }
        Data::Enum(ref data_enum) =>
            build_read_for_enum(&input.attrs, &input.ident, data_enum)?,
        Data::Union(_) => return Err(syn::Error::new(Span::call_site(), "Unions is not supported")),
    };
    let cp_crate = get_crate();
    let generics = add_trait_bounds(
        input.generics.clone(), vec![parse_quote! {#cp_crate::packet::PacketReadable}]);
    let (impl_generics, ty_generics, where_clause) =
        generics.split_for_impl();
    let ident = &input.ident;
    Ok(async_trait(
        quote! {
            impl #impl_generics #cp_crate::packet::PacketReadable for #ident #ty_generics #where_clause {
                async fn read(input: &mut impl #cp_crate::packet::InputPacketBytes) -> #cp_crate::packet::PacketReadableResult<Self> {
                    #func_body
                }
            }
        }
    ))
}