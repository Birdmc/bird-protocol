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
}

#[derive(Debug, Default)]
pub struct FieldAttributes {
    pub variant: Attribute<String>,
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
            path_to_string(path).as_str(), path.span()
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
        })
    }
}

impl Parse for FieldAttributes {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        FieldAttributes::try_from(input.parse::<Attributes>()?)
    }
}