use std::collections::HashMap;
use proc_macro2::{Ident, Span, TokenStream};
use quote::ToTokens;
use syn::{Lit, Expr, ExprPath, LitStr, Token};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;

pub type Attribute = Option<syn::Expr>;

#[derive(Default)]
pub struct PacketAttributes {
    pub id: Attribute,
    pub state: Attribute,
    pub side: Attribute,
    pub protocol: Attribute,
    pub writable: Attribute,
    pub readable: Attribute,
}

#[derive(Default)]
pub struct FieldAttributes {
    pub variant: Attribute,
    pub write: Attribute,
    pub read: Attribute,
}

#[derive(Default)]
pub struct EnumAttributes {
    pub variant: Attribute,
    pub primitive: Attribute,
}

#[derive(Default)]
pub struct Attributes {
    pub attributes: HashMap<String, Expr>,
}

pub fn path_to_string(path: &ExprPath) -> String {
    path.path.segments
        .iter()
        .map(|element| element.ident.to_string())
        .collect::<Vec<String>>()
        .join("::")
}

pub fn expr_to_lit(expr: &Expr) -> syn::Result<Lit> {
    match expr {
        Expr::Lit(ref lit) => Ok(lit.lit.clone()),
        Expr::Path(ref path) => Ok(Lit::Str(LitStr::new(
            path_to_string(path).as_str(), path.span(),
        ))),
        it => Err(syn::Error::new(it.span(), "Expected literal"))
    }
}

pub fn expr_to_string(expr: &Expr) -> syn::Result<String> {
    match expr_to_lit(expr)? {
        Lit::Str(str) => Ok({
            let str = str.token().to_string();
            str[1..str.len() - 1].to_string()
        }),
        it => Err(syn::Error::new(it.span(), "Expected string")),
    }
}

pub fn expr_to_bool(expr: &Expr) -> syn::Result<bool> {
    match expr_to_lit(expr)? {
        Lit::Bool(boolean) => Ok(boolean.value),
        it => Err(syn::Error::new(it.span(), "Expected boolean")),
    }
}

pub fn get_attribute(attributes: &Attributes,
                        names: Vec<String>,
) -> syn::Result<Attribute> {
    for name in names {
        match attributes.attributes.get(&name) {
            Some(expr) => return Ok(Some(expr.clone())),
            None => continue,
        }
    }
    Ok(None)
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut result = Attributes::default();
        while !input.is_empty() {
            let key = path_to_string(&input.parse::<ExprPath>()?);
            input.parse::<Token![=]>()?;
            let value: Expr = input.parse()?;
            result.attributes.insert(key, value);
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(result)
    }
}

impl TryFrom<Attributes> for PacketAttributes {
    type Error = syn::Error;

    fn try_from(attributes: Attributes) -> syn::Result<Self> {
        let attr = &attributes;
        Ok(PacketAttributes {
            id: get_attribute(attr, vec!["id".into()])?,
            state: get_attribute(attr, vec!["state".into()])?,
            side: get_attribute(attr, vec!["side".into()])?,
            protocol: get_attribute(attr, vec!["protocol".into()])?,
            readable: get_attribute(attr, vec!["readable".into()])?,
            writable: get_attribute(attr, vec!["writable".into()])?,
        })
    }
}

impl Parse for PacketAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        PacketAttributes::try_from(input.parse::<Attributes>()?)
    }
}

impl TryFrom<Attributes> for FieldAttributes {
    type Error = syn::Error;

    fn try_from(attributes: Attributes) -> syn::Result<Self> {
        let attr = &attributes;
        Ok(FieldAttributes {
            variant: get_attribute(attr, vec!["variant".into(), "var".into()])?,
            read: get_attribute(attr, vec!["read".into()])?,
            write: get_attribute(attr, vec!["write".into()])?,
        })
    }
}

impl Parse for FieldAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        FieldAttributes::try_from(input.parse::<Attributes>()?)
    }
}

impl TryFrom<Attributes> for EnumAttributes {
    type Error = syn::Error;

    fn try_from(attributes: Attributes) -> Result<Self, Self::Error> {
        let attr = &attributes;
        Ok(EnumAttributes {
            variant: get_attribute(attr, vec!["variant".into(), "var".into()])?,
            primitive: get_attribute(attr, vec!["primitive".into(), "pr".into()])?,
        })
    }
}

impl Parse for EnumAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        EnumAttributes::try_from(input.parse::<Attributes>()?)
    }
}

impl EnumAttributes {
    fn from_single(attribute: syn::Expr) -> syn::Result<Self> {
        let attribute_str = expr_to_string(&attribute)?;
        let primitive: String = match attribute_str.as_str() {
            "VarInt" => "i32",
            "VarLong" => "i64",
            other => other
        }.into();
        let variant_ident = Ident::new(attribute_str.as_str(), attribute.span());
        let primitive_ident = Ident::new(primitive.as_str(), attribute.span());
        Ok(Self {
            variant: Some(syn::Expr::Verbatim(variant_ident.to_token_stream())),
            primitive: Some(syn::Expr::Verbatim(primitive_ident.to_token_stream())),
        })
    }

    pub fn into_filled(self) -> syn::Result<Self> {
        if self.primitive.is_none() && self.variant.is_none() {
            return Err(syn::Error::new(Span::call_site(), "packet_enum should have primitive or variant variables"));
        }
        if self.primitive.is_some() && self.variant.is_some() {
            return Ok(self);
        }
        Ok(Self::from_single(match self.primitive.is_some() {
            true => self.primitive.unwrap(),
            false => self.variant.unwrap()
        })?)
    }

    pub fn find_one(attributes: &Vec<syn::Attribute>) -> syn::Result<Option<Self>> {
        attributes
            .iter()
            .find(|attr| attr.path.is_ident("packet_enum") || attr.path.is_ident("pe"))
            .map(|attr| attr.parse_args::<Self>())
            .map(|attr| attr.map(|val| Some(val)))
            .unwrap_or_else(|| Ok(None))
    }
}