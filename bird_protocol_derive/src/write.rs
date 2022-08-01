use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{DeriveInput, Field, Fields, Path};
use crate::util::{FieldAttributes, FieldVisitor, get_bird_protocol_crate, VariantAttributes, VariantVisitor, visit_derive_input, visit_fields};

pub struct WritableVariantVisitor {
    variants: TokenStream,
}

pub struct WritableFieldVisitor {
    fields: TokenStream,
    raw_writes: Vec<TokenStream>,
    ordered_writes: Vec<(usize, TokenStream)>,
}

impl WritableVariantVisitor {
    pub fn new() -> Self {
        Self {
            variants: quote! {}
        }
    }

    pub fn get_variants(&self) -> TokenStream {
        let Self { variants, .. } = self;
        quote! {
            #variants
            _ => unreachable!(),
        }
    }
}

impl VariantVisitor for WritableVariantVisitor {
    fn visit(&mut self, ident: Path, data_fields: &Fields, _attributes: VariantAttributes) -> syn::Result<()> {
        let Self { variants, .. } = self;
        let mut field_visitor = WritableFieldVisitor::new();
        visit_fields(data_fields, &mut field_visitor)?;
        let (fields, writes) = field_visitor.into_pieces();
        let fields = match data_fields {
            Fields::Unit => quote! {},
            Fields::Named(_) => quote! {{#fields}},
            Fields::Unnamed(_) => quote! {(#fields)},
        };
        *variants = quote! {
            #variants
            #ident #fields => { #(#writes)* },
        };
        Ok(())
    }
}

impl WritableFieldVisitor {
    pub fn new() -> Self {
        Self {
            fields: quote! {},
            raw_writes: vec![],
            ordered_writes: vec![],
        }
    }

    pub fn into_pieces(mut self) -> (TokenStream, Vec<TokenStream>) {
        self.ordered_writes
            .sort_by(|(index, _), (second_index, _)| index.cmp(second_index));
        self.ordered_writes
            .into_iter()
            .for_each(|(index, ts)| self.raw_writes.insert(index, ts));
        (self.fields, self.raw_writes)
    }
}

impl FieldVisitor for WritableFieldVisitor {
    fn visit(&mut self, ident: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()> {
        let Self {
            fields,
            raw_writes,
            ordered_writes, ..
        } = self;
        *fields = quote! {
            #fields
            ref #ident,
        };
        let protocol_crate = get_bird_protocol_crate();
        let Field { ty, .. } = field;
        let write_ts = match attributes.variant {
            Some(variant) => quote! {
                < #variant as #protocol_crate ::packet::PacketVariantWritable< #ty >>::write_variant( #ident , write)?;
            },
            None => quote! {
                < #ty as #protocol_crate ::packet::PacketWritable>::write( #ident , write)?;
            }
        };
        match attributes.order {
            Some(order) => ordered_writes.push((order, write_ts)),
            None => raw_writes.push(write_ts),
        }
        Ok(())
    }
}

pub fn write_impl(args: &DeriveInput) -> syn::Result<TokenStream> {
    let protocol_crate = get_bird_protocol_crate();
    let mut visitor = WritableVariantVisitor::new();
    visit_derive_input(args, &mut visitor)?;
    let DeriveInput { ident, generics, .. } = args;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let variants = visitor.get_variants();
    Ok(quote! {
        impl #impl_generics #protocol_crate ::packet::PacketWritable for #ident #ty_generics #where_clause {
            fn write<W>(&self, write: &mut W) -> Result<(), anyhow::Error>
                where W: #protocol_crate ::packet::PacketWrite {
                match self {
                    #variants
                }
                Ok(())
            }
        }
    })
}