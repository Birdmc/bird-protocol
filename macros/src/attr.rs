use std::borrow::Borrow;
use std::collections::HashMap;
use proc_macro2::Span;
use proc_macro_error::abort;
use syn::{Attribute, DeriveInput};
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use crate::util::{iterate_fields};

pub type AttributeValue<T> = Option<(T, Span)>;

#[derive(Debug, Default)]
pub struct HeaderAttributes {
    pub id: AttributeValue<i32>,
    pub state: AttributeValue<String>,
    pub side: AttributeValue<String>,
    pub protocol: AttributeValue<i32>,
}

#[derive(Debug, Default)]
pub struct FieldAttributes {
    pub order: AttributeValue<i32>,
    pub variant: AttributeValue<String>,
}

#[derive(Debug)]
pub enum ProtocolClass {
    Struct(ProtocolStruct),
    Enum(ProtocolEnum),
}

#[derive(Debug)]
pub struct ProtocolStruct {
    pub header: HeaderAttributes,
    pub fields: HashMap<String, FieldAttributes>,
}

#[derive(Debug)]
pub struct ProtocolEnum {
    pub types: HashMap<String, ProtocolStruct>,
}

struct PrAttribute {
    pub values: HashMap<String, syn::Lit>,
}

fn path_to_string(from: &syn::Path) -> String {
    from.segments.iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<String>>()
        .join("::")
}

fn lit_to_i32(from: syn::Lit) -> i32 {
    match from {
        syn::Lit::Int(int) => int.base10_parse()
            .map_err(|err| abort!(err.span(), "Expected: i32"))
            .unwrap(),
        it => abort!(it.span(), "Expected: i32"),
    }
}

fn lit_to_string(from: syn::Lit) -> String {
    match from {
        syn::Lit::Str(str) => str.value(),
        it => abort!(it.span(), "Expected: String"),
    }
}

impl From<&Attribute> for PrAttribute {
    fn from(attribute: &Attribute) -> Self {
        let mut result = PrAttribute {
            values: HashMap::new(),
        };

        let body: Punctuated<syn::ExprAssign, syn::Token![,]> =
            attribute.parse_args_with(|input: ParseStream| {
                let mut elems = Punctuated::new();
                while !input.is_empty() {
                    let first = input.parse()?;
                    elems.push_value(first);
                    if input.is_empty() {
                        break;
                    }
                    let punct = input.parse()?;
                    elems.push_punct(punct);
                }
                Ok(elems)
            })
                .map_err(|err| abort!(err.span(), "{}", err))
                .unwrap();
        body.into_iter()
            .map(|expr| {
                let key = match expr.left.borrow() {
                    syn::Expr::Path(path) => path_to_string(&path.path),
                    it => abort!(it.span(), "Expected: Path expr (Ident)"),
                };
                let value = match expr.right.borrow() {
                    syn::Expr::Lit(lit) => lit.lit.clone(),
                    syn::Expr::Path(path) => syn::Lit::Str(
                        syn::LitStr::new(path_to_string(&path.path).as_str(), path.span())
                    ),
                    it => abort!(it.span(), "Expected: Literal or Path (Ident)"),
                };
                (key, value, expr.span())
            })
            .for_each(|(key, value, span)|
                match result.values.insert(key, value) {
                    Some(_) => abort!(span, "Duplicate assignment"),
                    _ => {}
                }
            );
        result
    }
}

fn iterate_attribute(
    attributes: &Vec<Attribute>,
    idents: Vec<&str>,
    mut iterate_f: impl FnMut(Span, String, syn::Lit),
) {
    attributes
        .iter()
        .filter(|attribute| idents.iter()
            .any(|str| attribute.path.is_ident(str))
        )
        .map(|attribute| (attribute.span(), PrAttribute::from(attribute).values))
        .for_each(|(span, values)| values
            .into_iter()
            .for_each(|(key, value)| iterate_f(span, key, value))
        );
}

impl From<&Vec<Attribute>> for HeaderAttributes {
    fn from(attributes: &Vec<Attribute>) -> Self {
        let mut result = HeaderAttributes::default();
        iterate_attribute(
            attributes,
            vec!["packet"],
            |span, key, value| match key.as_str() {
                "id" => result.id = Some((lit_to_i32(value), span)),
                "state" => result.state = Some((lit_to_string(value), span)),
                "side" => result.side = Some((lit_to_string(value), span)),
                "protocol" => result.protocol = Some((lit_to_i32(value), span)),
                it => abort!(span, format!("Unknown key: {}", it)),
            },
        );
        result
    }
}

impl From<&Vec<Attribute>> for FieldAttributes {
    fn from(attributes: &Vec<Attribute>) -> Self {
        let mut result = FieldAttributes::default();
        iterate_attribute(
            attributes,
            vec!["protocol_field", "pf"],
            |span, key, value| match key.as_str() {
                "order" => result.order = Some((lit_to_i32(value), span)),
                "variant" | "var" => result.variant = Some((lit_to_string(value), span)),
                it => abort!(span, format!("Unknown key: {}", it)),
            },
        );
        result
    }
}

impl From<(&Vec<Attribute>, &syn::Fields)> for ProtocolStruct {
    fn from(from: (&Vec<Attribute>, &syn::Fields)) -> Self {
        let (header_attributes, struct_fields) = from;
        let header = HeaderAttributes::from(header_attributes);
        let mut fields = HashMap::new();
        iterate_fields(
            &struct_fields,
            |name, field| {
                fields.insert(name, FieldAttributes::from(&field.attrs));
            },
        );
        ProtocolStruct { header, fields }
    }
}

impl From<&DeriveInput> for ProtocolClass {
    fn from(input: &DeriveInput) -> Self {
        match input.data {
            syn::Data::Struct(ref data_struct) => ProtocolClass::Struct(
                ProtocolStruct::from((&input.attrs, &data_struct.fields))
            ),
            syn::Data::Enum(ref data_enum) => {
                let mut types = HashMap::new();
                data_enum.variants
                    .iter()
                    .for_each(|variant| {
                        types.insert(
                            variant.ident.to_string(),
                            ProtocolStruct::from((&variant.attrs, &variant.fields)),
                        );
                    });
                ProtocolClass::Enum(ProtocolEnum { types })
            }
            syn::Data::Union(_) => abort!(input.span(), "Union type is not supported"),
        }
    }
}