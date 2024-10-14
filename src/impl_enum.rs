use proc_macro2::Span;
use syn::*;
use quote::quote;
use crate::{field_attributes::FieldAttributes, output::Output, root_attributes::RootAttributes};

pub fn process_enum(data_enum: &mut DataEnum, root_attributes: &RootAttributes, output: &mut Output) {
    let mut lines = vec![];
    let mut impl_display_lines = vec![];
    let mut get_location_lines = vec![];
    let mut completion_suggestions = vec![];
    let has_name = root_attributes.name.is_some();

    for i in 0..data_enum.variants.len() {
        let variant = &mut data_enum.variants[i];

        // TODO: check if variant should be skipped to avoid recursion

        let variant_name = &variant.ident;
        let variant_name_as_str = variant_name.to_string();
        let attributes = FieldAttributes::from_field_attributes(&mut variant.attrs);
        let mut parse_prefix = quote! { true };
        let mut parse_suffix = quote! { true };
        let mut parse_method = quote! { parse_item(reader__) };
        let mut line = quote! { };
        let mut pass_marker_test_fragments = vec![];

        for marker_name in &attributes.ignore_if_marker {
            pass_marker_test_fragments.push(quote! {
                !reader__.get_marker(#marker_name)
            });
        }

        for marker_name in &attributes.ignore_if_not_marker {
            pass_marker_test_fragments.push(quote! {
                reader__.get_marker(#marker_name)
            });
        }

        let pass_marker_test = match pass_marker_test_fragments.is_empty() {
            true => quote! { true },
            false => quote! { #(#pass_marker_test_fragments)&&* }
        };

        let (field_markers_on_start, field_markers_on_exit, field_markers_on_fail) = attributes.get_push_pop_markers(i);

        if let Some(prefix) = attributes.prefix {
            let prefix_consume_spaces = match attributes.consume_spaces_after_prefix {
                Some(false) => quote! { {} },
                _ => quote! { reader__.eat_spaces() },
            };

            parse_prefix = quote! {
                match reader__.read_string(#prefix) {
                    Some(_) => { #prefix_consume_spaces; true },
                    None => { reader__.set_expected_string(#prefix); false }
                }
            };
        }

        if let Some(suffix) = attributes.suffix {
            let suffix_consume_spaces = match attributes.consume_spaces_after_suffix {
                Some(false) => quote! { {} },
                _ => quote! { reader__.eat_spaces() },
            };

            parse_suffix = quote! {
                match reader__.read_string(#suffix) {
                    Some(_) => { #suffix_consume_spaces; true },
                    None => { reader__.set_expected_string(#suffix); false }
                }
            };
        }

        if let Some(separator) = attributes.separator {
            parse_method = quote! { parse_item_with_separator(reader__, #separator) };
        }

        match &variant.fields {
            Fields::Named(_) => unreachable!(),
            Fields::Unnamed(fields_unnamed) => {
                let mut value_names = vec![];

                for i in 0..fields_unnamed.unnamed.len() {
                    let value_name = Ident::new(&format!("value_{}", i), Span::call_site());

                    value_names.push(quote! { #value_name });
                }

                let mut current_block_single = quote! {
                    let suffix_ok__ = #parse_suffix;

                    if suffix_ok__ {
                        #field_markers_on_exit
                        return Some(Self::#variant_name(#(#value_names),*))
                    }
                };

                for (i, field) in fields_unnamed.unnamed.iter().enumerate().rev() {
                    let field_type = &field.ty;
                    let value_name = Ident::new(&format!("value_{}", i), Span::call_site());
                    let consume_spaces = match attributes.consume_spaces {
                        Some(false) => quote! { },
                        _ => quote! { reader__.eat_spaces(); },
                    };

                    value_names.insert(0, quote! { #value_name });

                    current_block_single = quote! {
                        if let Some(#value_name) = <#field_type as parsable::Parsable>::#parse_method {
                            #consume_spaces
                            #current_block_single
                        }
                    };
                }

                line = quote! {
                    let prefix_ok__ = #parse_prefix;

                    if prefix_ok__ {
                        #current_block_single
                    }

                    reader__.set_index(start_index__);
                };

                if fields_unnamed.unnamed.len() == 1 {
                    let field = &fields_unnamed.unnamed[0];
                    let field_type = &field.ty;

                    get_location_lines.push(quote! {
                        Self::#variant_name(value) => <#field_type as parsable::Parsable>::location(value),
                    });
                } else {
                    let mut fields = vec![];
                    
                    for _ in 0..fields_unnamed.unnamed.len() {
                        fields.push(quote! { _ });
                    }
                    get_location_lines.push(quote! {
                        Self::#variant_name(#(#fields),*) => panic!("variant `{}` has no location (because it doesn't have exactly 1 field)", #variant_name_as_str),
                    });
                }
            },
            Fields::Unit => {
                let string = match &variant.discriminant {
                    Some((_, Expr::Lit(expr_lit))) => {
                        match &expr_lit.lit {
                            Lit::Str(value) => {
                                Some(value)
                            },
                            _ => None
                        }
                    },
                    _ => None
                };

                get_location_lines.push(quote! {
                    Self::#variant_name => panic!("variant `{}` has no location (because it doesn't have exactly 1 field)", #variant_name_as_str),
                });

                match string {
                    Some(lit_str) => {
                        completion_suggestions.push(lit_str.clone());
                        line = quote! {
                            if let Some(_) = reader__.read_string(#lit_str) {
                                reader__.eat_spaces();
                                #field_markers_on_exit
                                return Some(Self::#variant_name);
                            } else if (! #has_name) {

                                reader__.set_expected_string(#lit_str);
                            }
                        };

                        impl_display_lines.push(quote! {
                            Self::#variant_name => #lit_str,
                        });
                    },
                    None => {
                        // emit_call_site_error!("variants with no field must have an associated string literal")
                    }
                }
            }
        }

        lines.push(quote! {
            if (#pass_marker_test) {
                #field_markers_on_start
                #line
                #field_markers_on_fail
                #field_markers_on_exit
            }
        });
    }

    for variant in data_enum.variants.iter_mut() {
        variant.discriminant = None;
    }

    if root_attributes.impl_display {
        output.display = Some(quote! {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let string = match self {
                    #(#impl_display_lines)*
                    _ => "<?>"
                };

                write!(f, "{}", string)
            }
        });
    }

    output.as_str = Some(quote! {
        pub fn as_str(&self) -> &'static str {
            match self {
                #(#impl_display_lines)*
                _ => ""
            }
        }
    });

    output.parse_item = quote! {
        fn parse_item(reader__: &mut parsable::StringReader) -> Option<Self> {
            let start_index__ = reader__.get_index();
            #(#lines)*

            None
        }
    };

    output.get_location = quote! {
        fn location(&self) -> &parsable::ItemLocation {
            match self {
                #(#get_location_lines)*
            }
        }
    };

    output.get_completion_suggestions = Some(quote! {
        fn get_completion_suggestions() -> &'static[&'static str] {
            &[ #(#completion_suggestions),* ]
        }
    });
}