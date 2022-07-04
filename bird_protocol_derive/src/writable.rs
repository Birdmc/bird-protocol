use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{DeriveInput, Field, Fields, Data, parse_quote, Variant};
use syn::spanned::Spanned;
use crate::attribute::{EnumAttributes, FieldAttributes};
use crate::c_enum::{EnumVariantVisitor, visit_enum_variants};
use crate::fields::{CollectFieldVisitor, EnumFields, FieldVisitor, visit_fields};
use crate::util::{add_trait_bounds, async_trait, get_crate};

pub struct WritableVisitor {
    object_ts: TokenStream,
    row_order: Vec<TokenStream>,
    referencing: TokenStream,
}

impl WritableVisitor {
    pub fn new(object_ts: TokenStream, referencing: bool) -> WritableVisitor {
        WritableVisitor {
            object_ts,
            row_order: Vec::new(),
            referencing: match referencing {
                true => quote!{&},
                false => quote!{}
            }
        }
    }

    pub fn get_result(self) -> TokenStream {
        TokenStream::from_iter(self.row_order)
    }

    pub fn get_result_with_return(self) -> TokenStream {
        let result = self.get_result();
        quote! {
            #result
            Ok(())
        }
    }
}

impl FieldVisitor for WritableVisitor {
    fn visit(&mut self, name: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()> {
        let WritableVisitor { object_ts, referencing, .. } = self;
        let writable_value = match attributes.write.or(attributes.variant) {
            Some(variant) => {
                (
                    quote! {#variant},
                    quote! {<#variant>::from(#referencing #object_ts #name)},
                )
            }
            None => {
                let field_ty = &field.ty;
                (
                    quote! {#field_ty},
                    quote! {#object_ts #name},
                )
            }
        };
        let writable = write_ts(writable_value.0, writable_value.1);
        self.row_order.push(writable);
        Ok(())
    }
}

pub struct EnumVariantWritableVisitor {
    pub in_match: TokenStream,
    pub primitive: TokenStream,
    pub variant: TokenStream,
}

impl EnumVariantVisitor for EnumVariantWritableVisitor {
    fn visit(&mut self, variant: &Variant, value: TokenStream) -> syn::Result<()> {
        let cp_crate = get_crate();
        let in_match = &self.in_match;
        let primitive = &self.primitive;
        let variant_ident = &variant.ident;
        let mut collect_fields = CollectFieldVisitor::new();
        visit_fields(&variant.fields, &mut collect_fields)?;
        let collected_fields = collect_fields.get();
        let mut field_idents = collected_fields
            .iter()
            .map(|(ident, _, _)| ident.clone())
            .collect::<Vec<Ident>>();
        let match_el_header = match variant.fields {
            Fields::Unit => quote! {Self::#variant_ident},
            Fields::Named(_) => {
                quote! {
                    Self::#variant_ident {
                        #(#field_idents,)*
                    }
                }
            }
            Fields::Unnamed(_) => {
                field_idents = field_idents
                    .into_iter()
                    .map(|ident| Ident::new(format!("__{}", ident).as_str(), ident.span()))
                    .collect::<Vec<Ident>>();
                quote! {
                    Self::#variant_ident (
                        #( #field_idents ,)*
                    )
                }
            }
        };
        let mut writable_visitor = WritableVisitor::new(quote! {}, false);
        for (ident, field, attrs) in collected_fields {
            writable_visitor.visit(ident, &field, attrs)?;
        }
        let result_ts = writable_visitor.get_result();
        let variant = &self.variant;
        self.in_match = quote! {
            #in_match
            #match_el_header => {
                <#variant as #cp_crate::packet::PacketWritable>::write(
                        &#variant::from((#value) as #primitive), output
                ).await?;
                #result_ts
            },
        };
        Ok(())
    }
}

pub fn write_ts(ty: TokenStream, value: TokenStream) -> TokenStream {
    let cp_crate = get_crate();
    quote! {<#ty as #cp_crate::packet::PacketWritable>::write(& #value, output).await?;}
}

pub fn build_writable_function_body(input: &DeriveInput) -> syn::Result<TokenStream> {
    Ok(match input.data {
        Data::Struct(ref data_struct) => {
            let mut visitor = WritableVisitor::new(quote! {self.}, true);
            visit_fields(&data_struct.fields, &mut visitor)?;
            visitor.get_result_with_return()
        }
        Data::Enum(ref data_enum) => match EnumAttributes::find_one(&input.attrs)? {
            Some(attrs) => {
                let attrs = attrs.into_filled()?;
                let primitive = attrs.primitive.unwrap().to_token_stream();
                let variant = attrs.variant.unwrap().to_token_stream();
                let mut visitor = EnumVariantWritableVisitor { in_match: quote! {}, primitive, variant };
                visit_enum_variants(&mut visitor, &input.ident, data_enum)?;
                let in_match = visitor.in_match;
                quote! {
                    match self {
                        #in_match
                    }
                    Ok(())
                }
            }
            None => {
                let mut variants = TokenStream::new();
                for variant in &data_enum.variants {
                    let enum_fields = EnumFields::build(&variant.fields)?;
                    let mut writable_visitor = WritableVisitor::new(
                        enum_fields.prefix(), false);
                    visit_fields(&variant.fields, &mut writable_visitor)?;
                    let variant_ident = &variant.ident;
                    let variant_arguments = enum_fields.arguments();
                    let variant_body = writable_visitor.get_result_with_return();
                    variants = quote! {
                    #variants
                    match Self::#variant_ident #variant_arguments => {
                        #variant_body
                    },
                }
                }
                match variants.is_empty() {
                    true => quote! { Ok(()) },
                    false => quote! {
                        match self {
                            #variants
                        }
                    },
                }
            }
        }
        Data::Union(_) => return Err(
            syn::Error::new(input.span(), "Union type is not supported")
        )
    })
}

pub fn writable_trait_from_body(input: &DeriveInput, func_body: TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let ident = &input.ident;
    let cp_crate = get_crate();
    let generics = add_trait_bounds(
        input.generics.clone(),
        vec![parse_quote! {#cp_crate::packet::PacketWritable}],
    );
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    Ok(async_trait(quote! {
        impl #impl_generics #cp_crate::packet::PacketWritable for #ident #ty_generics #where_clause {
            async fn write(&self, output: &mut impl #cp_crate::packet::OutputPacketBytes) -> #cp_crate::packet::PacketWritableResult {
                #func_body
            }
        }
    }))
}

pub fn writable_macro_impl(input: &DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    let func_body = build_writable_function_body(&input)?;
    writable_trait_from_body(input, func_body)
}