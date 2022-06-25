use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{DeriveInput, Field, Fields, Data, parse_quote, DataEnum, Attribute};
use syn::spanned::Spanned;
use crate::attribute::{EnumAttributes, FieldAttributes};
use crate::c_enum::{CEnumFieldsVisitor, is_c_enum, visit_c_enum};
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

struct CEnumReadVisitor {
    counter: usize,
    values: TokenStream,
    matches: TokenStream,
    primitive: TokenStream,
}

impl CEnumReadVisitor {
    pub fn new(primitive: TokenStream) -> Self {
        Self {
            counter: 0,
            values: TokenStream::new(),
            matches: TokenStream::new(),
            primitive,
        }
    }
}

impl CEnumFieldsVisitor for CEnumReadVisitor {
    fn visit(&mut self, name: &Ident, value: TokenStream) {
        let Self { counter, values, matches, primitive } = self;
        let value_ident = Ident::new(format!("__{}", counter).as_str(), value.span());
        *counter += 1;
        *matches = quote! {
            #matches
            #value_ident => Ok(Self::#name),
        };
        *values = quote! {
            #values
            const #value_ident: #primitive = (#value) as #primitive;
        };
    }
}

pub fn build_read_for_enum(attrs: &Vec<Attribute>, data_enum: &DataEnum) -> syn::Result<TokenStream> {
    match is_c_enum(data_enum) {
        true => {
            let enum_attributes: EnumAttributes = match attrs
                .iter()
                .find(|attr| attr.path.is_ident("pe") || attr.path.is_ident("packet_enum"))
                .map(|attr| attr.parse_args()) {
                Some(attr) => attr?,
                None => return Err(syn::Error::new(Span::call_site(), "didn't found packet_enum attribute")),
            };
            let cp_crate = get_crate();
            let enum_attributes = enum_attributes.into_filled()?;
            let (value, span) = enum_attributes.primitive.unwrap();
            let primitive = Ident::new(value.as_str(), span);
            let (value, span) = enum_attributes.variant.unwrap();
            let variant = Ident::new(value.as_str(), span);
            let mut variant_visitor = CEnumReadVisitor::new(primitive.to_token_stream());
            visit_c_enum(data_enum, &mut variant_visitor)?;
            let CEnumReadVisitor { values, matches, .. } = variant_visitor;
            Ok(quote! {
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
        Data::Enum(ref data_enum) => build_read_for_enum(&input.attrs, data_enum)?,
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