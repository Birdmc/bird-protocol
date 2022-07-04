use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{DeriveInput, Field, Fields, Data, parse_quote, DataEnum, Attribute, Variant};
use crate::attribute::{EnumAttributes, FieldAttributes};
use crate::c_enum::{EnumVariantVisitor, visit_enum_variants};
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
    match attributes.read.as_ref().or(attributes.variant.as_ref()) {
        Some(expr) => {
            let read_st = read_ts(quote! { #expr });
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

struct EnumVariantReadableVisitor {
    pub consts: TokenStream,
    pub matches: TokenStream,
    primitive: TokenStream,
    counter: usize,
}

impl EnumVariantReadableVisitor {
    pub fn new(primitive: TokenStream) -> Self {
        Self {
            consts: TokenStream::new(),
            matches: TokenStream::new(),
            counter: 0,
            primitive,
        }
    }
}

impl EnumVariantVisitor for EnumVariantReadableVisitor {
    fn visit(&mut self, variant: &Variant, value: TokenStream) -> syn::Result<()> {
        let consts = &self.consts;
        let matches = &self.matches;
        let primitive_ty= &self.primitive;
        let variant_ident = &variant.ident;
        let const_ident = Ident::new(format!("__{}", self.counter).as_str(), Span::call_site());
        self.counter += 1;
        self.consts = quote! {
            #consts
            const #const_ident: #primitive_ty = (#value) as #primitive_ty;
        };
        let variant_read = build_read_for(&variant.fields)?;
        self.matches = quote! {
            #matches
            #const_ident => Self:: #variant_ident #variant_read,
        };
        Ok(())
    }
}

pub fn build_read_for_enum(attrs: &Vec<Attribute>, enum_ident: &Ident, data_enum: &DataEnum) -> syn::Result<TokenStream> {
    match EnumAttributes::find_one(&attrs)? {
        Some(attrs) => {
            let attrs = attrs.into_filled()?;
            let cp_crate = get_crate();
            let primitive = attrs.primitive.to_token_stream();
            let variant = attrs.variant.to_token_stream();
            let mut readable_visitor = EnumVariantReadableVisitor::new(primitive.clone());
            visit_enum_variants(&mut readable_visitor, enum_ident, data_enum)?;
            let consts = readable_visitor.consts;
            let matches = readable_visitor.matches;
            Ok(quote! {
                let value: #primitive = <#primitive>::from(
                    <#variant as #cp_crate::packet::PacketReadable>::read(input).await?
                );
                #consts
                Ok(match value {
                    #matches
                    _ => return Err(#cp_crate::packet::PacketReadableError::Custom(
                        #cp_crate::packet::CustomError::StaticStr("Bad enum value")
                    ))
                })
            })
        }
        None => return Err(syn::Error::new(
            Span::call_site(), "didn't found packet_enum attribute")),
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