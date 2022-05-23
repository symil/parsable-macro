use proc_macro2::{TokenStream};

#[derive(Default)]
pub struct Output {
    pub display: Option<TokenStream>,
    pub as_str: Option<TokenStream>,
    pub get_location: TokenStream,
    pub parse_item: TokenStream,
    pub token_name: TokenStream,
    pub get_completion_suggestions: Option<TokenStream>
}