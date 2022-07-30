use std::collections::HashMap;
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{quote, ToTokens};
use syn::{Attribute, Data, DataEnum, DataStruct, DeriveInput, Expr, Field, Fields, GenericParam, Generics, LifetimeDef, Lit, parse_quote, Path, PathArguments, PathSegment, Token};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Colon2, Type};

const FIELD_ATTRIBUTES: &[&str] = &["variant", "var", "order"];
const DATA_ATTRIBUTES: &[&str] = &[];

#[derive(Debug, Clone)]
pub struct FieldAttributes {
    pub order: Option<usize>,
    pub variant: Option<TokenStream>,
}

#[derive(Debug, Clone)]
pub struct DataAttributes {}

pub trait FieldVisitor {
    fn visit(&mut self, ident: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()>;
}

pub trait VariantVisitor {
    fn visit(&mut self, ident: Path, fields: &Fields, attributes: DataAttributes) -> syn::Result<()>;
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
                    Ident::new(counter.to_string().as_str(), field.span()),
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
    attributes: &Vec<Attribute>,
    data_enum: &DataEnum,
    visitor: &mut impl VariantVisitor,
) -> syn::Result<()> {
    for variant in &data_enum.variants {
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
            get_attributes(DATA_ATTRIBUTES, attributes)?.try_into()?,
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
        get_attributes(DATA_ATTRIBUTES, attributes)?.try_into()?,
    )
}

fn get_attributes<'a>(names: &[&'a str], attributes: &Vec<Attribute>) -> syn::Result<HashMap<&'a str, Expr>> {
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

fn expr_to_usize(expr: &Expr) -> syn::Result<usize> {
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

    fn try_from(_: HashMap<&str, Expr>) -> Result<Self, Self::Error> {
        Ok(DataAttributes {})
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

pub fn add_trait_bounds(mut generics: Generics, bounds: &[TokenStream]) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            for bound in bounds {
                type_param.bounds.push(parse_quote! { #bound })
            }
        }
    }
    generics
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