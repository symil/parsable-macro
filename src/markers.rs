use quote::quote;
use proc_macro2::TokenStream;
use syn::LitStr;
use crate::utils::make_ident;

pub struct MarkerOutput {
    pub on_parse_start: TokenStream,
    pub on_parse_exit: TokenStream,
    pub on_parse_fail: TokenStream,
}

impl MarkerOutput {
    pub fn from_attributes(declared_markers: &[LitStr], set_markers: &[LitStr], unset_markers: &[LitStr], field_index: Option<usize>) -> Self {
        let mut start = vec![];
        let mut exit = vec![];
        let mut fail = vec![];

        let prefix = match field_index {
            Some(index) => format!("field_{}_", index),
            None => String::new(),
        };

        for marker in declared_markers {
            let marker_str = marker.value().replace("-", "_");
            let var_ident = make_ident(format!("{}{}_id", prefix, marker_str));

            start.push(quote! { let #var_ident = reader__.declare_marker(#marker); });
            exit.insert(0, quote! { reader__.remove_marker(#var_ident); });
        }

        for (marker_list, value) in [set_markers, unset_markers].iter().zip(&[true, false]) {
            for marker in *marker_list {
                let marker_str = marker.value().replace("-", "_");
                let var_ident = make_ident(format!("{}{}_value", prefix, marker_str));

                start.push(quote! { let #var_ident = reader__.set_marker(#marker, #value); });
                fail.insert(0, quote! { reader__.set_marker(#marker, #var_ident); })
            }
        }

        Self {
            on_parse_start: quote! { #(#start)* },
            on_parse_exit: quote! { #(#exit)* },
            on_parse_fail: quote! { #(#fail)* },
        }
    }

    pub fn to_tuple(self) -> (TokenStream, TokenStream, TokenStream) {
        (
            self.on_parse_start,
            self.on_parse_exit,
            self.on_parse_fail,
        )
    }
}