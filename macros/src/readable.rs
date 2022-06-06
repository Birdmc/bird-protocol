use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, Field, Fields, Data, parse_quote};
use crate::attribute::FieldAttributes;
use crate::fields::{FieldVisitor, visit_fields};
use crate::util::{add_trait_bounds, async_trait, get_crate};

pub trait ReadVisitor {
    fn get_body(&self) -> TokenStream;
}

struct UnnamedReadVisitor {
    fields: TokenStream,
}

struct NamedReadVisitor {
    fields: TokenStream,
}

impl UnnamedReadVisitor {
    pub fn new() -> Self {
        UnnamedReadVisitor {
            fields: quote! {},
        }
    }
}

impl ReadVisitor for UnnamedReadVisitor {
    fn get_body(&self) -> TokenStream {
        let UnnamedReadVisitor { fields } = self;
        quote! {
            (#fields)
        }
    }
}

impl FieldVisitor for UnnamedReadVisitor {
    fn visit(&mut self, name: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()> {
        let UnnamedReadVisitor { fields } = self;
        let variable = read_ts_variant(&name, field, &attributes);
        self.fields = quote! {
            #fields
            #variable,
        };
        Ok(())
    }
}

impl NamedReadVisitor {
    pub fn new() -> Self {
        NamedReadVisitor {
            fields: quote! {}
        }
    }
}

impl ReadVisitor for NamedReadVisitor {
    fn get_body(&self) -> TokenStream {
        let NamedReadVisitor { fields } = self;
        quote! {
            {
                #fields
            }
        }
    }
}

impl FieldVisitor for NamedReadVisitor {
    fn visit(&mut self, name: Ident, field: &Field, attributes: FieldAttributes) -> syn::Result<()> {
        let NamedReadVisitor { fields } = self;
        let variable = read_ts_variant(&name, &field, &attributes);
        self.fields = quote! {
            #fields
            #name: #variable,
        };
        Ok(())
    }
}

pub fn read_ts(ty: TokenStream) -> TokenStream {
    let cp_crate = get_crate();
    quote! { <#ty as #cp_crate::packet::PacketReadable>::read(input).await? }
}

fn read_ts_variant(_ident: &Ident, field: &Field, attributes: &FieldAttributes) -> TokenStream {
    match &attributes.variant {
        Some((variant, span)) => {
            let ident = Ident::new(variant.as_str(), span.clone());
            let read_st = read_ts(quote! { #ident });
            quote! { #read_st .into() }
        }
        None => {
            let field_ty = &field.ty;
            read_ts(quote! { #field_ty })
        }
    }
}

pub fn build_read_for(fields: &Fields) -> syn::Result<TokenStream> {
    Ok(match fields {
        Fields::Unit => quote! {},
        Fields::Named(_) => {
            let mut visitor = NamedReadVisitor::new();
            visit_fields(fields, &mut visitor)?;
            visitor.get_body()
        }
        Fields::Unnamed(_) => {
            let mut visitor = UnnamedReadVisitor::new();
            visit_fields(fields, &mut visitor)?;
            visitor.get_body()
        }
    })
}

pub fn readable_macro_impl(input: &DeriveInput) -> syn::Result<proc_macro::TokenStream> {
    let func_body = match input.data {
        Data::Struct(ref data_struct) => {
            let read = build_read_for(&data_struct.fields)?;
            quote! {
                Ok(Self #read)
            }
        }
        _ => return Err(syn::Error::new(Span::call_site(), "Only struct type is supported"))
    };
    let cp_crate = get_crate();
    let generics = add_trait_bounds(
        input.generics.clone(), vec![parse_quote! {#cp_crate::packet::PacketReadable}]);
    let (impl_generics, ty_generics, where_clause) =
        generics.split_for_impl();
    let ident = &input.ident;
    Ok(async_trait(
        quote! {
            impl #impl_generics #cp_crate::packet::PacketReadable for #ident #ty_generics #where_clause {
                async fn read(input: &mut impl #cp_crate::packet::InputPacketBytes) -> #cp_crate::packet::PacketReadableResult<Self> {
                    #func_body
                }
            }
        }
    ))
}