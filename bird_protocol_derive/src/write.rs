use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{DeriveInput, Field, Fields, Path};
use crate::util::{DATA_ATTRIBUTES, DataAttributes, FieldAttributes, FieldVisitor, get_attributes, get_bird_protocol_crate, VariantAttributes, VariantVisitor, visit_derive_input, visit_fields};

pub struct WritableVariantVisitor {
    variants: TokenStream,
    data_attributes: DataAttributes,
}

pub struct WritableFieldVisitor {
    fields: TokenStream,
    raw_writes: Vec<TokenStream>,
    ordered_writes: Vec<(usize, TokenStream)>,
}

impl WritableVariantVisitor {
    pub fn new(data_attributes: DataAttributes) -> Self {
        Self {
            data_attributes,
            variants: quote! {},
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
    fn visit(&mut self, ident: Path, data_fields: &Fields,
             value: Option<TokenStream>, _attributes: VariantAttributes) -> syn::Result<()> {
        let Self { variants, .. } = self;
        let mut field_visitor = WritableFieldVisitor::new();
        visit_fields(data_fields, &mut field_visitor)?;
        let (fields, writes) = field_visitor.into_pieces();
        let fields = match data_fields {
            Fields::Unit => quote! {},
            Fields::Named(_) => quote! {{#fields}},
            Fields::Unnamed(_) => quote! {(#fields)},
        };

        let variant = &self.data_attributes.enum_variant;
        let ty = &self.data_attributes.enum_type;

        let end_writes;
        match value {
            Some(value) => {
                if variant.is_none() && ty.is_none() {
                    end_writes = quote! { #(#writes)* };
                } else if variant.is_some() && ty.is_some() {
                    let ty = ty.as_ref().unwrap();
                    let write_ts = write_ts(
                        variant,
                        ty,
                        &quote! { &((#value) as #ty) },
                    );
                    end_writes = quote! {
                        #write_ts
                        #( #writes )*
                    }
                } else {
                    let end_variant = variant.as_ref().or(ty.as_ref()).unwrap().clone();
                    let write_ts = write_ts(
                        &None,
                        &end_variant,
                        &quote! { &((#value) as #end_variant) },
                    );
                    end_writes = quote! {
                        #write_ts
                        #( #writes )*
                    }
                }
            }
            None => end_writes = quote! { #( #writes )* },
        }

        *variants = quote! {
            #variants
            #ident #fields => { #end_writes },
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
        let Field { ty, .. } = field;
        let write_ts = write_ts(
            &attributes.variant,
            &quote! { #ty },
            &ident.to_token_stream(),
        );
        match attributes.order {
            Some(order) => ordered_writes.push((order, write_ts)),
            None => raw_writes.push(write_ts),
        }
        Ok(())
    }
}

fn write_ts(variant: &Option<TokenStream>, ty: &TokenStream, value: &TokenStream) -> TokenStream {
    let protocol_crate = get_bird_protocol_crate();
    match variant {
        Some(ref variant) => quote! {
            < #variant as #protocol_crate ::packet::PacketVariantWritable< #ty >>
            ::write_variant( #value , write)?;
        },
        None => quote! {
            < #ty as #protocol_crate ::packet::PacketWritable>::write( #value , write)?;
        }
    }
}

pub fn write_impl(args: &DeriveInput) -> syn::Result<TokenStream> {
    let data_attributes =
        get_attributes(DATA_ATTRIBUTES, &args.attrs)?.try_into()?;
    let protocol_crate = get_bird_protocol_crate();
    let mut visitor = WritableVariantVisitor::new(data_attributes);
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