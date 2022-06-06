use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Field, Fields};
use syn::spanned::Spanned;

use crate::attribute::FieldAttributes;

pub trait FieldVisitor {
    fn visit(&mut self, name: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()>;
}

pub fn visit_fields(input: &Fields, visitor: &mut impl FieldVisitor) -> syn::Result<()> {
    let mut counter = -1;
    for field in input {
        let attrs = field.attrs
            .iter()
            .find(|attr| attr.path.is_ident("pf"))
            .map(|attr| attr.parse_args())
            .unwrap_or_else(|| Ok(FieldAttributes::default()))?;
        match field.ident {
            Some(ref ident) => visitor.visit(ident.clone(), field, attrs),
            None => {
                counter += 1;
                visitor.visit(
                    Ident::new(counter.to_string().as_str(), field.span()),
                    field, attrs,
                )
            }
        }?
    }
    Ok(())
}

pub struct CollectFieldVisitor {
    visited: Vec<(Ident, Field, FieldAttributes)>,
}

impl CollectFieldVisitor {
    pub fn new() -> CollectFieldVisitor {
        CollectFieldVisitor { visited: Vec::new() }
    }

    pub fn get(self) -> Vec<(Ident, Field, FieldAttributes)> {
        self.visited
    }
}

impl FieldVisitor for CollectFieldVisitor {
    fn visit(&mut self, name: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()> {
        self.visited.push((name, field.clone(), attributes));
        Ok(())
    }
}

pub enum FieldsType {
    Named,
    Unnamed,
    Unit,
}

pub struct EnumFields {
    fields: Vec<Ident>,
    fields_type: FieldsType,
}

impl EnumFields {
    pub fn build(fields: &Fields) -> syn::Result<EnumFields> {
        let mut collect = CollectFieldVisitor::new();
        visit_fields(fields, &mut collect)?;
        Ok(EnumFields {
            fields: collect
                .get()
                .into_iter()
                .map(|(ident, ..)| ident)
                .collect(),
            fields_type: match fields {
                Fields::Unnamed(_) => FieldsType::Unnamed,
                Fields::Named(_) => FieldsType::Named,
                Fields::Unit => FieldsType::Unit,
            }
        })
    }

    pub fn prefix(&self) -> TokenStream {
        match self.fields_type {
            FieldsType::Unnamed => quote! {__},
            _ => quote! {},
        }
    }

    pub fn string_prefix(&self) -> &'static str {
        match self.fields_type {
            FieldsType::Unnamed => "__",
            _ => ""
        }
    }

    pub fn arguments(&self) -> TokenStream {
        let prefix = self.string_prefix();
        let formatted_idents: Vec<Ident> = self.fields
            .iter()
            .map(|field| Ident::new(
                format!("{}{}", prefix, field.to_string()).as_str(),
                field.span(),
            ))
            .collect();
        match self.fields_type {
            FieldsType::Named => quote!{{#(#formatted_idents),*}},
            FieldsType::Unnamed => quote!{(#(#formatted_idents),*)},
            FieldsType::Unit => quote!{}
        }
    }
}