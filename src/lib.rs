#![allow(unused_assignments)]

mod root_attributes;
mod field_attributes;
mod utils;
mod impl_struct;
mod impl_enum;
mod output;
mod markers;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::*;
use proc_macro_error::*;

use root_attributes::*;
use impl_struct::*;
use impl_enum::*;

use crate::output::Output;

// https://docs.rs/syn/latest/syn/struct.DeriveInput.html
#[proc_macro_error]
#[proc_macro_attribute]
pub fn parsable(attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut root_attributes = match syn::parse::<RootAttributes>(attr.clone()) {
        Ok(attributes) => attributes,
        Err(error) => {
            emit_call_site_error!(error);
            RootAttributes::default()
        }
    };
    let mut ast : DeriveInput = syn::parse(input).unwrap();
    let mut output = Output::default();
    let name = &ast.ident;

    let derive_attribute = syn::Attribute {
        pound_token: Token![#](Span::call_site()),
        style: AttrStyle::Outer,
        bracket_token: syn::token::Bracket { span: Span::call_site() },
        path: syn::parse_str("derive").unwrap(),
        tokens: syn::parse_str("(Debug)").unwrap(),
    };

    // if let Data::Enum(_) = ast.data {
    //     derive_attribute.tokens = syn::parse_str("(Debug, Clone, Copy, PartialEq)").unwrap();
    // }

    ast.attrs.push(derive_attribute);

    match &mut ast.data {
        Data::Struct(data) => process_struct(data, &mut root_attributes, &mut output),
        Data::Enum(data) => process_enum(data, &root_attributes, &mut output),
        Data::Union(_) => emit_call_site_error!("unions are not supported")
    }

    let impl_display = match output.display {
        Some(body) => quote! {
            impl std::fmt::Display for #name {
                #body
            }
        },
        None => quote! {},
    };

    let impl_as_str = match output.as_str {
        Some(body) => quote! {
            impl #name {
                #body
            }
        },
        None => quote! {}
    };

    let token_name = match &root_attributes.name {
        Some(specified_name) => specified_name.to_string(),
        None => match &root_attributes.token {
            Some(token) => format!("\"{}\"", token),
            None => name.to_string(),
        }
    };
    let token_name_lit = LitStr::new(&token_name, Span::call_site());

    let impl_item_name = quote! {
        fn get_item_name() -> String {
            #token_name_lit.to_string()
        }
    };

    let get_location = output.get_location;
    let parse_item = output.parse_item;
    let get_completion_suggestions = output.get_completion_suggestions.unwrap_or_default();

    let result = quote! {
        #ast

        impl parsable::Parsable for #name {
            #parse_item

            #impl_item_name

            #get_location

            #get_completion_suggestions
        }

        impl std::ops::Deref for #name {
            type Target = parsable::ItemLocation;

            fn deref(&self) -> &parsable::ItemLocation {
                <Self as parsable::Parsable>::location(self)
            }
        }

        #impl_display

        #impl_as_str
    };

    result.into()
}