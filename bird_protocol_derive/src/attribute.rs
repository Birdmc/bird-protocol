use std::collections::HashMap;
use proc_macro2::Span;
use syn::{Lit, Expr, ExprPath, LitStr, Token};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;

pub type Attribute<T> = Option<(T, Span)>;

#[derive(Debug, Default)]
pub struct PacketAttributes {
    pub id: Attribute<i32>,
    pub state: Attribute<String>,
    pub side: Attribute<String>,
    pub protocol: Attribute<i32>,
    pub writable: Attribute<bool>,
    pub readable: Attribute<bool>,
}

#[derive(Debug, Default)]
pub struct FieldAttributes {
    pub variant: Attribute<String>,
    pub write: Attribute<String>,
    pub read: Attribute<String>,
    pub write_lifetime: Attribute<bool>,
}

#[derive(Debug, Default)]
pub struct EnumAttributes {
    pub variant: Attribute<String>,
    pub primitive: Attribute<String>,
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

pub fn expr_to_int(expr: &Expr) -> syn::Result<i32> {
    match expr_to_lit(expr)? {
        Lit::Int(int) => int.base10_parse(),
        it => Err(syn::Error::new(it.span(), "Expected i32")),
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

pub fn get_attribute<T>(attributes: &Attributes,
                        names: Vec<String>,
                        mut parse: impl FnMut(&Expr) -> syn::Result<T>) -> syn::Result<Attribute<T>> {
    for name in names {
        match attributes.attributes.get(&name) {
            Some(expr) => return Ok(Some(
                (parse(expr)?, expr.span())
            )),
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
            id: get_attribute(
                attr, vec!["id".into()], expr_to_int)?,
            state: get_attribute(
                attr, vec!["state".into()], expr_to_string)?,
            side: get_attribute(
                attr, vec!["side".into()], expr_to_string)?,
            protocol: get_attribute(
                attr, vec!["protocol".into()], expr_to_int)?,
            readable: get_attribute(
                attr, vec!["readable".into()], expr_to_bool)?,
            writable: get_attribute(
                attr, vec!["writable".into()], expr_to_bool)?,
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
            variant: get_attribute(
                attr, vec!["variant".into(), "var".into()], expr_to_string)?,
            read: get_attribute(
                attr, vec!["read".into()], expr_to_string)?,
            write: get_attribute(
                attr, vec!["write".into()], expr_to_string)?,
            write_lifetime: get_attribute(
                attr, vec!["write_lifetime".into(), "wl".into()], expr_to_bool)?
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
            variant: get_attribute(
                attr, vec!["variant".into(), "var".into()], expr_to_string)?,
            primitive: get_attribute(
                attr, vec!["primitive".into(), "pr".into()], expr_to_string)?,
        })
    }
}

impl Parse for EnumAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        EnumAttributes::try_from(input.parse::<Attributes>()?)
    }
}

impl EnumAttributes {
    fn from_one(attribute: (String, Span)) -> Self {
        let (value, span) = attribute;
        let primitive = match value.as_str() {
            "VarInt" => "i32",
            "VarLong" => "i64",
            other => other
        }.into();
        Self {
            variant: Some((value, span)),
            primitive: Some((primitive, span.clone())),
        }
    }

    pub fn into_filled(self) -> syn::Result<Self> {
        if self.primitive.is_none() && self.variant.is_none() {
            return Err(syn::Error::new(Span::call_site(), "packet_enum should have primitive or variant variables"));
        }
        if self.primitive.is_some() && self.variant.is_some() {
            return Ok(self);
        }
        Ok(Self::from_one(match self.primitive.is_some() {
            true => self.primitive.unwrap(),
            false => self.variant.unwrap()
        }))
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