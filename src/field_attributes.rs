use proc_macro2::TokenStream;
use proc_macro_error::emit_call_site_error;
use syn::{*, parse::{Parse, ParseStream}};
use crate::markers::MarkerOutput;

#[derive(Default)]
pub struct FieldAttributes {
    pub value: Option<String>,
    pub regex: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub min: Option<usize>,
    pub separator: Option<String>,
    pub optional: Option<bool>,
    pub cascade: Option<bool>,
    pub consume_spaces: Option<bool>,
    pub consume_spaces_after_prefix: Option<bool>,
    pub consume_spaces_after_suffix: Option<bool>,
    pub consume_spaces_between_items: Option<bool>,
    pub exclude: Option<String>,
    pub followed_by: Option<String>,
    pub not_followed_by: Option<String>,
    pub declared_markers: Vec<LitStr>,
    pub set_markers: Vec<LitStr>,
    pub unset_markers: Vec<LitStr>,
    pub ignore_if_marker: Vec<LitStr>,
    pub ignore_if_not_marker: Vec<LitStr>,
    pub ignore: bool,
}

impl Parse for FieldAttributes {
    #[allow(unused_must_use)]
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut attributes = FieldAttributes::default();
        let content;

        parenthesized!(content in input);

        while !content.is_empty() {
            let name = content.parse::<Ident>()?.to_string();

            if name.as_str() == "ignore" {
                attributes.ignore = true;
            } else {
                content.parse::<Token![=]>()?;

                match name.as_str() {
                    "value" => attributes.value = Some(content.parse::<LitStr>()?.value()),
                    "regex" => attributes.regex = Some(content.parse::<LitStr>()?.value()),
                    "prefix" => attributes.prefix = Some(content.parse::<LitStr>()?.value()),
                    "suffix" => attributes.suffix = Some(content.parse::<LitStr>()?.value()),
                    "brackets" => {
                        let brackets = content.parse::<LitStr>()?.value();

                        if brackets.len() == 2 {
                            attributes.prefix = Some((brackets.as_bytes()[0] as char).to_string());
                            attributes.suffix = Some((brackets.as_bytes()[1] as char).to_string());
                        }
                    },
                    "min" => attributes.min = Some(content.parse::<LitInt>()?.base10_parse::<usize>()?),
                    "sep" => attributes.separator = Some(content.parse::<LitStr>()?.value()),
                    "separator" => attributes.separator = Some(content.parse::<LitStr>()?.value()),
                    "optional" => attributes.optional = Some(content.parse::<LitBool>()?.value()),
                    "cascade" => attributes.cascade = Some(content.parse::<LitBool>()?.value()),
                    "followed_by" => attributes.followed_by = Some(content.parse::<LitStr>()?.value()),
                    "not_followed_by" => attributes.not_followed_by = Some(content.parse::<LitStr>()?.value()),
                    "exclude" => attributes.exclude = Some(content.parse::<LitStr>()?.value()),
                    "declare_marker" => attributes.declared_markers.push(content.parse::<LitStr>()?),
                    "set_marker" => attributes.set_markers.push(content.parse::<LitStr>()?),
                    "unset_marker" => attributes.unset_markers.push(content.parse::<LitStr>()?),
                    "ignore_if_marker" => attributes.ignore_if_marker.push(content.parse::<LitStr>()?),
                    "ignore_if_not_marker" => attributes.ignore_if_not_marker.push(content.parse::<LitStr>()?),
                    "consume_spaces" => attributes.consume_spaces = Some(content.parse::<LitBool>()?.value()),
                    "consume_spaces_after_prefix" => attributes.consume_spaces_after_prefix = Some(content.parse::<LitBool>()?.value()),
                    "consume_spaces_after_suffix" => attributes.consume_spaces_after_suffix = Some(content.parse::<LitBool>()?.value()),
                    "consume_spaces_between_items" => attributes.consume_spaces_between_items = Some(content.parse::<LitBool>()?.value()),
                    _ => {}
                }
            }

            if !content.is_empty() {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(attributes)
    }
}

impl FieldAttributes {
    pub fn from_field_attributes(attrs: &mut Vec<Attribute>) -> Self {
        let mut attributes = Self::default();

        if let Some((i, attr)) = attrs.iter().enumerate().find(|(_, attr)| attr.path.segments.last().unwrap().ident == "parsable") {
            let result = syn::parse2::<FieldAttributes>(attr.tokens.clone());

            match result {
                Ok(value) => attributes = value,
                Err(error) => emit_call_site_error!(error)
            };

            attrs.remove(i);
        }

        attributes
    }

    pub fn get_push_pop_markers(&self, field_index: usize) -> (TokenStream, TokenStream, TokenStream) {
        MarkerOutput::from_attributes(&self.declared_markers, &self.set_markers, &self.unset_markers, Some(field_index)).to_tuple()
    }
}