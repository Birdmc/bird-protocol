use proc_macro2::{Span, TokenStream, Ident};
use quote::quote;
use syn::{DeriveInput, Fields};
use syn::spanned::Spanned;
use crate::attr::ProtocolStruct;
use crate::ProtocolClass;
use crate::util::{add_trait_to_generics, collect_types, get_crate, default_use};

pub fn writable_impl(mut input: DeriveInput) -> TokenStream {
    let protocol_class = ProtocolClass::from(&input);
    let writable_lines = match protocol_class {
        ProtocolClass::Struct(ref protocol_struct) => match input.data {
            syn::Data::Struct(data_struct) => writable_lines(
                &data_struct.fields, protocol_struct, quote! {self.}),
            _ => unreachable!()
        },
        ProtocolClass::Enum(protocol_enum) => match input.data {
            syn::Data::Enum(data_enum) => {
                let variants: Vec<TokenStream> = data_enum.variants
                    .into_iter()
                    .map(|variant| {
                        let writable_lines = writable_lines(
                            &variant.fields,
                            protocol_enum.types
                                .get(&variant.ident.to_string())
                                .unwrap(),
                            match variant.fields {
                                Fields::Unnamed(_) => quote! {__obj},
                                _ => TokenStream::new(),
                            },
                        );
                        let enum_ident = &input.ident;
                        let variant_ident = variant.ident;
                        let field_idents = match variant.fields {
                            Fields::Unit => Vec::new(),
                            Fields::Unnamed(ref unnamed) => {
                                let mut counter = -1;
                                unnamed.unnamed
                                    .iter()
                                    .map(|field| {
                                        counter += 1;
                                        syn::Ident::new(
                                            format!("f_{}", counter).as_str(), field.span(),
                                        )
                                    })
                                    .collect()
                            }
                            Fields::Named(ref named) => named.named
                                .iter()
                                .map(|field| field.ident
                                    .as_ref().unwrap().clone()
                                )
                                .collect()
                        };
                        let fields = match variant.fields {
                            Fields::Unit => TokenStream::new(),
                            Fields::Named(_) => quote! {(#(#field_idents),*)},
                            Fields::Unnamed(_) => quote! {{#(#field_idents),*}},
                        };
                        let pre_write = match variant.fields {
                            Fields::Unnamed(_) => quote! {let __obj = (#(#field_idents),*);},
                            _ => TokenStream::new(),
                        };
                        quote! {
                            match #enum_ident::#variant_ident #fields {
                                #pre_write
                                #writable_lines
                            }
                        }
                    })
                    .collect();
                quote! {
                    match self {
                        #(#variants),*
                    }
                }
            }
            _ => unreachable!()
        }
    };
    let writable_generics = add_trait_to_generics(input.generics, quote! { $crate::packet::PacketWritable });
    let (impl_generics, ty_generics, where_clause) =
        writable_generics.split_for_impl();
    let name = input.ident;
    let cp_crate = get_crate();
    let default_use = default_use();
    quote! {
        #[async_trait::async_trait]
        impl #impl_generics #cp_crate::packet::PacketWritable for #name #ty_generics #where_clause {
            async fn write(self, output: &mut impl #cp_crate::packet::OutputPacketBytes) ->
                #cp_crate::packet::PacketWritableResult {
                #default_use
                #writable_lines
                Ok(())
            }
        }
    }
}

fn writable_lines(fields: &Fields, protocol_struct: &ProtocolStruct, obj_ts: TokenStream) -> TokenStream {
    TokenStream::from_iter(
        collect_types(fields, protocol_struct)
            .into_iter()
            .map(|(name, type_ts)| {
                let name_ident = Ident::new(name.as_str(), Span::call_site());
                quote! {
                    #type_ts::from(#obj_ts #name_ident).write(output).await?;
                }
            })
    )
}