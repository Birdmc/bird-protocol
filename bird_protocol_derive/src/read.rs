use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Field, Fields, Path};
use crate::util::{DATA_ATTRIBUTES, DataAttributes, FieldAttributes, FieldVisitor, get_attributes, get_bird_protocol_crate, get_lifetimes, VariantAttributes, VariantVisitor, visit_derive_input, visit_fields};

pub struct ReadableVariantVisitor {
    pub lifetime: TokenStream,
    pub variant_creators: Vec<TokenStream>,
}

pub struct ReadableFieldVisitor {
    raw_reads: Vec<TokenStream>,
    ordered_reads: Vec<(usize, TokenStream)>,
    values: Vec<TokenStream>,
    named: bool,
    lifetime: TokenStream,
}

impl VariantVisitor for ReadableVariantVisitor {
    fn visit(&mut self, ident: Path, fields: &Fields, _: VariantAttributes) -> syn::Result<()> {
        match fields {
            Fields::Unit => self.variant_creators.push(quote! { #ident }),
            _ => {
                let named = match fields {
                    Fields::Named(_) => true,
                    _ => false
                };
                let mut field_visitor = ReadableFieldVisitor::new(
                    named, self.lifetime.clone(),
                );
                visit_fields(fields, &mut field_visitor)?;
                let (reads, values) = field_visitor.into_pieces();
                let values = match named {
                    true => quote! { { #( #values, )* } },
                    false => quote! { ( #( #values, )* ) },
                };
                self.variant_creators.push(quote! {
                    #( #reads; )*
                    #ident #values
                })
            }
        }
        Ok(())
    }
}

impl ReadableFieldVisitor {
    pub fn new(named: bool, lifetime: TokenStream) -> Self {
        Self {
            raw_reads: vec![],
            ordered_reads: vec![],
            values: vec![],
            named,
            lifetime,
        }
    }

    pub fn into_pieces(mut self) -> (Vec<TokenStream>, Vec<TokenStream>) {
        self.ordered_reads
            .sort_by(|(index, _), (second_index, _)| index.cmp(second_index));
        self.ordered_reads
            .into_iter()
            .for_each(|(index, ts)| self.raw_reads.insert(index, ts));
        (self.raw_reads, self.values)
    }
}

impl FieldVisitor for ReadableFieldVisitor {
    fn visit(&mut self, ident: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()> {
        let value_ident = Ident::new(
            format!("__{}", ident.to_string()).as_str(), ident.span(),
        );
        let value_read = read_statement(field, &attributes, &self.lifetime)?;
        let read = quote! { let #value_ident = #value_read };
        match attributes.order {
            Some(index) => self.ordered_reads.push((index, read)),
            None => self.raw_reads.push(read)
        }
        self.values.push(match self.named {
            true => quote! { #ident : #value_ident },
            false => quote! { #value_ident }
        });
        Ok(())
    }
}

pub fn read_statement(field: &Field, attributes: &FieldAttributes, lifetime: &TokenStream) -> syn::Result<TokenStream> {
    let Field { ty, .. } = field;
    let protocol_crate = get_bird_protocol_crate();
    Ok(match attributes.variant {
        Some(ref variant) => quote! {
            < #variant as #protocol_crate ::packet::PacketVariantReadable< #lifetime , #ty >>
            ::read_variant(read)?
        },
        None => quote! {
            < #ty as #protocol_crate ::packet::PacketReadable< #lifetime >>::read(read)?
        }
    })
}

pub fn read_impl(args: &DeriveInput) -> syn::Result<TokenStream> {
    match args.data {
        Data::Struct(_) => {
            let data_attributes: DataAttributes =
                get_attributes(DATA_ATTRIBUTES, &args.attrs)?.try_into()?;
            let lifetime = match data_attributes.lead_lifetime {
                Some(lifetime) => lifetime,
                None => {
                    let lifetimes = get_lifetimes(&args.generics);
                    match lifetimes.len() {
                        0 => quote! {'_},
                        1 => lifetimes.get(0).unwrap().to_token_stream(),
                        _ => return Err(syn::Error::new(
                            Span::call_site(), "attribute data with lifetime to set lead lifetime"
                        ))
                    }
                }
            };
            let mut variant_visitor = ReadableVariantVisitor {
                lifetime: lifetime.clone(),
                variant_creators: vec![]
            };
            visit_derive_input(args, &mut variant_visitor)?;
            let DeriveInput { ident, generics, .. } = args;
            let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
            let variants = variant_visitor.variant_creators;
            let protocol_crate = get_bird_protocol_crate();
            Ok(quote! {
                impl #impl_generics #protocol_crate ::packet::PacketReadable< #lifetime > for #ident #ty_generics #where_clause {
                    fn read<R>(read: &mut R) -> Result<Self, #protocol_crate ::packet::PacketReadableError>
                        where R: #protocol_crate ::packet::PacketRead< #lifetime > {
                        std::result::Result::Ok({ #(#variants)* })
                    }
                }
            })
        }
        Data::Enum(_) =>
            Err(syn::Error::new(Span::call_site(), "enum type is not supported, yet")),
        Data::Union(_) =>
            Err(syn::Error::new(Span::call_site(), "union type is not supported"))
    }
}