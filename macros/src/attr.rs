use std::borrow::Borrow;
use std::collections::HashMap;
use proc_macro2::Span;
use proc_macro_error::abort;
use syn::{Attribute, DeriveInput};
use syn::spanned::Spanned;

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

pub struct ProtocolStruct {
    pub header: HeaderAttributes,
    pub fields: HashMap<String, FieldAttributes>,
    pub input: DeriveInput,
}

struct PrAttribute {
    pub name: String,
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
            name: path_to_string(&attribute.path),
            values: HashMap::new(),
        };
        let body: syn::ExprArray = attribute.parse_args()
            .map_err(|err| abort!(err.span(), "{}", err))
            .unwrap();
        body.elems
            .into_iter()
            .map(|expr| match expr {
                syn::Expr::Assign(assign) => assign,
                it => abort!(it.span(), "Expected: Assign"),
            })
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

impl From<DeriveInput> for ProtocolStruct {
    fn from(input: DeriveInput) -> Self {
        let header = HeaderAttributes::from(&input.attrs);
        let mut fields = HashMap::new();
        match input.data {
            syn::Data::Struct(ref data_struct) => match data_struct.fields {
                syn::Fields::Named(ref struct_fields) => struct_fields.named
                    .iter()
                    .for_each(|field| {
                        fields.insert(
                            field.ident.as_ref().unwrap().to_string(), FieldAttributes::from(&field.attrs),
                        );
                    }),
                syn::Fields::Unnamed(ref struct_fields) => {
                    let mut counter = 0;
                    struct_fields.unnamed
                        .iter()
                        .for_each(|field| {
                            fields.insert(
                                format!("{}", counter), FieldAttributes::from(&field.attrs),
                            );
                            counter += 1;
                        })
                }
                syn::Fields::Unit => {}
            },
            _ => abort!(input.span(), "Only struct types supported"),
        }
        ProtocolStruct { fields, header, input }
    }
}