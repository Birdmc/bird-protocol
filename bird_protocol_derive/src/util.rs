use std::collections::HashMap;
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{quote, ToTokens};
use syn::{Attribute, Data, DataEnum, DataStruct, DeriveInput, Expr, Field, Fields, GenericParam, Generics, LifetimeDef, Lit, parse_quote, Path, PathArguments, PathSegment};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Colon2;

pub const FIELD_ATTRIBUTES: &[&str] = &["variant", "var", "order"];
pub const DATA_ATTRIBUTES: &[&str] = &["lifetime", "enum_type", "enum_variant"];
pub const VARIANT_ATTRIBUTES: &[&str] = &["value"];

#[derive(Debug, Clone)]
pub struct FieldAttributes {
    pub order: Option<usize>,
    pub variant: Option<TokenStream>,
}

#[derive(Debug, Clone)]
pub struct DataAttributes {
    pub enum_type: Option<TokenStream>,
    pub enum_variant: Option<TokenStream>,
    pub lead_lifetime: Option<TokenStream>,
}

#[derive(Debug, Clone)]
pub struct VariantAttributes {
    pub value: Option<TokenStream>,
}

pub trait FieldVisitor {
    fn visit(&mut self, ident: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()>;
}

pub trait VariantVisitor {
    fn visit(&mut self, ident: Path, fields: &Fields, value: Option<TokenStream>, attributes: VariantAttributes) -> syn::Result<()>;
}

pub fn visit_fields(fields: &Fields, visitor: &mut impl FieldVisitor) -> syn::Result<()> {
    match fields {
        Fields::Named(named) => for field in &named.named {
            visitor.visit(
                field.ident.clone().unwrap(),
                field,
                get_attributes(FIELD_ATTRIBUTES, &field.attrs)?.try_into()?,
            )?;
        },
        Fields::Unnamed(unnamed) => {
            let mut counter = 0usize;
            for field in &unnamed.unnamed {
                visitor.visit(
                    Ident::new(format!("__{}", counter).as_str(), field.span()),
                    field,
                    get_attributes(FIELD_ATTRIBUTES, &field.attrs)?.try_into()?,
                )?;
                if counter != usize::MAX {
                    counter += 1;
                }
            }
        }
        Fields::Unit => {}
    }
    Ok(())
}

pub fn visit_derive_input(derive_input: &DeriveInput, visitor: &mut impl VariantVisitor) -> syn::Result<()> {
    match derive_input.data {
        Data::Struct(ref data_struct) =>
            visit_struct(&derive_input.ident, &derive_input.attrs, data_struct, visitor),
        Data::Enum(ref data_enum) =>
            visit_enum(&derive_input.ident, &derive_input.attrs, data_enum, visitor),
        Data::Union(ref data_union) => Err(syn::Error::new(
            data_union.union_token.span, "Union types is not supported, yet. Use enum instead",
        ))
    }
}

pub fn visit_enum(
    enum_ident: &Ident,
    _attributes: &Vec<Attribute>,
    data_enum: &DataEnum,
    visitor: &mut impl VariantVisitor,
) -> syn::Result<()> {
    let mut start = quote! { 0 };
    let mut counter = -1isize;
    for variant in &data_enum.variants {
        let variant_attributes: VariantAttributes =
            get_attributes(VARIANT_ATTRIBUTES, &variant.attrs)?.try_into()?;
        match variant_attributes.value.as_ref().or(
            variant.discriminant.as_ref().map(|(_, expr)| expr.to_token_stream()).as_ref()
        ) {
            Some(value) => {
                start = value.clone();
                counter = 0;
            }
            None => {
                counter += 1;
            }
        }
        visitor.visit(
            Path {
                leading_colon: None,
                segments: {
                    let mut res = Punctuated::new();
                    res.push_value(PathSegment {
                        ident: enum_ident.clone(),
                        arguments: PathArguments::None,
                    });
                    res.push_punct(Colon2::default());
                    res.push_value(PathSegment {
                        ident: variant.ident.clone(),
                        arguments: PathArguments::None,
                    });
                    res
                },
            },
            &variant.fields,
            Some(quote! { #start + #counter }),
            variant_attributes,
        )?
    }
    Ok(())
}

pub fn visit_struct(
    struct_ident: &Ident,
    attributes: &Vec<Attribute>,
    data_struct: &DataStruct,
    visitor: &mut impl VariantVisitor,
) -> syn::Result<()> {
    visitor.visit(
        Path {
            leading_colon: None,
            segments: {
                let mut res = Punctuated::new();
                res.push_value(PathSegment {
                    ident: struct_ident.clone(),
                    arguments: PathArguments::None,
                });
                res
            },
        },
        &data_struct.fields,
        None,
        get_attributes(DATA_ATTRIBUTES, attributes)?.try_into()?,
    )
}

pub fn get_attributes<'a>(names: &[&'a str], attributes: &Vec<Attribute>) -> syn::Result<HashMap<&'a str, Expr>> {
    let mut res = HashMap::new();
    for attribute in attributes {
        for name in names {
            if attribute.path.is_ident(name) {
                res.insert(*name, attribute.parse_args()?);
                break;
            }
        }
    }
    Ok(res)
}

pub fn expr_to_usize(expr: &Expr) -> syn::Result<usize> {
    if let Expr::Lit(ref lit) = expr {
        if let Lit::Int(ref int) = lit.lit {
            return int.base10_parse();
        }
    }
    Err(syn::Error::new(expr.span(), "Must be positive integer"))
}

impl TryFrom<HashMap<&str, Expr>> for FieldAttributes {
    type Error = syn::Error;

    fn try_from(value: HashMap<&str, Expr>) -> Result<Self, Self::Error> {
        Ok(FieldAttributes {
            order: match value.get("order") {
                Some(expr) => Some(expr_to_usize(expr)?),
                None => None,
            },
            variant: value.get("variant")
                .or(value.get("var"))
                .map(|expr| expr.to_token_stream()),
        })
    }
}

impl TryFrom<HashMap<&str, Expr>> for DataAttributes {
    type Error = syn::Error;

    fn try_from(value: HashMap<&str, Expr>) -> Result<Self, Self::Error> {
        Ok(DataAttributes {
            enum_type: value.get("enum_type")
                .map(|expr| expr.to_token_stream()),
            enum_variant: value.get("enum_variant")
                .map(|expr| expr.to_token_stream()),
            lead_lifetime: value.get("lifetime")
                .map(|expr| expr.to_token_stream()),
        })
    }
}

impl TryFrom<HashMap<&str, Expr>> for VariantAttributes {
    type Error = syn::Error;

    fn try_from(attrs: HashMap<&str, Expr>) -> Result<Self, Self::Error> {
        Ok(VariantAttributes {
            value: attrs.get("value")
                .map(|expr| expr.to_token_stream())
        })
    }
}

pub fn get_bird_protocol_crate() -> TokenStream {
    let found_crate = crate_name("bird-protocol").unwrap();
    match found_crate {
        FoundCrate::Itself => quote! {crate},
        FoundCrate::Name(name) => {
            let ident = Ident::new(name.as_str(), Span::call_site());
            quote! {#ident}
        }
    }
}

pub fn add_trait_lifetime(generics: &mut Generics, lifetime: TokenStream) {
    generics.params.push(parse_quote! { #lifetime })
}

pub fn get_lifetimes(generics: &Generics) -> Vec<LifetimeDef> {
    let mut result = Vec::new();
    for param in &generics.params {
        if let GenericParam::Lifetime(ref lifetime) = param {
            result.push(lifetime.clone())
        }
    }
    result
}