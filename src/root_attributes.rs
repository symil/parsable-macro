use proc_macro2::TokenStream;
use syn::{*, parse::{Parse, ParseStream}};
use crate::markers::MarkerOutput;

// TODO: add prefix and suffix
pub struct RootAttributes {
    pub located: bool,
    pub impl_display: bool,
    pub cascade: bool,
    pub name: Option<String>,
    pub token: Option<String>,
    pub declared_markers: Vec<LitStr>,
    pub set_markers: Vec<LitStr>,
    pub unset_markers: Vec<LitStr>,
    pub ignore_if_marker: Vec<LitStr>,
    pub ignore_if_not_marker: Vec<LitStr>,
}

impl Default for RootAttributes {
    fn default() -> Self {
        Self {
            located: true,
            impl_display: false,
            cascade: false,
            name: None,
            token: None,
            declared_markers: vec![],
            set_markers: vec![],
            unset_markers: vec![],
            ignore_if_marker: vec![],
            ignore_if_not_marker: vec![],
        }
    }
}

impl Parse for RootAttributes {
    fn parse(content: ParseStream) -> syn::Result<Self> {
        let mut attributes = RootAttributes::default();

        while !content.is_empty() {
            let name = content.parse::<Ident>()?.to_string();
            content.parse::<Token![=]>()?;

            match name.as_str() {
                "located" => attributes.located = content.parse::<LitBool>()?.value(),
                "impl_display" => attributes.impl_display = content.parse::<LitBool>()?.value(),
                "cascade" => attributes.cascade = content.parse::<LitBool>()?.value(),
                "name" => attributes.name = Some(content.parse::<LitStr>()?.value()),
                "declare_marker" => attributes.declared_markers.push(content.parse::<LitStr>()?),
                "set_marker" => attributes.set_markers.push(content.parse::<LitStr>()?),
                "unset_marker" => attributes.unset_markers.push(content.parse::<LitStr>()?),
                "ignore_if_marker" => attributes.ignore_if_marker.push(content.parse::<LitStr>()?),
                "ignore_if_not_marker" => attributes.ignore_if_not_marker.push(content.parse::<LitStr>()?),
                _ => {}
            }

            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(attributes)
    }
}

impl RootAttributes {
    pub fn get_push_pop_markers(&self) -> (TokenStream, TokenStream, TokenStream) {
        MarkerOutput::from_attributes(&self.declared_markers, &self.set_markers, &self.unset_markers, None).to_tuple()
    }
}