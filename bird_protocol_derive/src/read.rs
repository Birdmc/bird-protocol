use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Field, Fields, Path};
use syn::spanned::Spanned;
use crate::util::{add_trait_lifetime, DATA_ATTRIBUTES, DataAttributes, FieldAttributes, FieldVisitor, get_attributes, get_bird_protocol_crate, get_lifetimes, VariantAttributes, VariantVisitor, visit_derive_input, visit_fields};

pub struct ReadableVariantVisitor {
    pub data_attributes: DataAttributes,
    pub lifetime: TokenStream,
    pub variant_creators: Vec<(Option<TokenStream>, TokenStream)>,
}

pub struct ReadableFieldVisitor {
    raw_reads: Vec<TokenStream>,
    ordered_reads: Vec<(usize, TokenStream)>,
    values: Vec<TokenStream>,
    named: bool,
    lifetime: TokenStream,
}

impl VariantVisitor for ReadableVariantVisitor {
    fn visit(&mut self, ident: Path, fields: &Fields, value: Option<TokenStream>, _: VariantAttributes) -> syn::Result<()> {
        match fields {
            Fields::Unit => self.variant_creators.push((value, quote! { #ident })),
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
                self.variant_creators.push(
                    (
                        value,
                        quote! {
                            #( #reads; )*
                            #ident #values
                        }
                    )
                )
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
        let Field { ty, .. } = field;
        let value_read = read_statement(&quote! {#ty}, &attributes.variant, &self.lifetime)?;
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

pub fn read_statement(ty: &TokenStream, variant: &Option<TokenStream>, lifetime: &TokenStream) -> syn::Result<TokenStream> {
    let protocol_crate = get_bird_protocol_crate();
    Ok(match variant {
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
    let data_attributes: DataAttributes =
        get_attributes(DATA_ATTRIBUTES, &args.attrs)?.try_into()?;
    let (add_lifetime, lifetime) = match data_attributes.lead_lifetime {
        Some(ref lifetime) => (false, lifetime.clone()),
        None => {
            let lifetimes = get_lifetimes(&args.generics);
            match lifetimes.len() {
                0 => {
                    (true, quote! {'a})
                }
                1 => (false, lifetimes.get(0).unwrap().to_token_stream()),
                _ => return Err(syn::Error::new(
                    Span::call_site(), "attribute data with lifetime to set lead lifetime",
                ))
            }
        }
    };
    if let Data::Union(_) = args.data {
        return Err(syn::Error::new(Span::call_site(), "union type is not supported"));
    }
    let mut variant_visitor = ReadableVariantVisitor {
        data_attributes,
        lifetime: lifetime.clone(),
        variant_creators: vec![],
    };
    visit_derive_input(args, &mut variant_visitor)?;
    let protocol_crate = get_bird_protocol_crate();
    let body: TokenStream = match args.data {
        Data::Struct(_) => {
            let (_, variants) = variant_visitor.variant_creators.get(0).unwrap();
            quote! {std::result::Result::Ok({ #variants })}
        }
        Data::Enum(_) => {
            if variant_visitor.data_attributes.enum_type.is_none() && variant_visitor.data_attributes.enum_variant.is_none() {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "You should provide enum type and variant to use PacketReadable macro",
                ));
            }
            let (ty, variant) = match variant_visitor.data_attributes.enum_type {
                Some(ref enum_type) => (enum_type, &variant_visitor.data_attributes.enum_variant),
                None => (variant_visitor.data_attributes.enum_variant.as_ref().unwrap(), &None)
            };
            let value_read_ts = read_statement(
                ty,
                variant,
                &lifetime,
            )?;
            let mut values = quote! {};
            let mut counter = 0usize;
            let mut result = quote! {};
            for (value, variant) in variant_visitor.variant_creators {
                let value = value.unwrap(); // it is enum
                let value_ident = Ident::new(format!("__{}", counter).as_str(), value.span());
                counter += 1;
                values = quote! {
                    #values
                    const #value_ident: #ty = #value as #ty;
                };
                result = quote! {
                    #result
                    #value_ident => {
                        #variant
                    },
                }
            }
            quote! {
                let __value = #value_read_ts;
                #values
                std::result::Result::Ok(match __value {
                    #result
                    _ => return std::result::Result::Err(
                        #protocol_crate ::packet::PacketReadableError::Any(anyhow::Error::msg("Bad value for enum"))
                    )
                })
            }
        }
        _ => unreachable!()
    };
    let DeriveInput { ident, generics, .. } = args;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let mut cloned_generics = generics.clone();
    let impl_generics = match add_lifetime {
        true => {
            add_trait_lifetime(&mut cloned_generics, quote! {'a});
            cloned_generics.split_for_impl().0
        }
        false => impl_generics,
    };
    Ok(quote! {
        impl #impl_generics #protocol_crate ::packet::PacketReadable< #lifetime > for #ident #ty_generics #where_clause {
            fn read<R>(read: &mut R) -> Result<Self, #protocol_crate ::packet::PacketReadableError>
            where R: #protocol_crate ::packet::PacketRead< #lifetime > {
                #body
            }
        }
    })
}