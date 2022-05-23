use syn::{*, parse::{Parse, ParseStream}};
use quote::quote;
use crate::{field_attributes::FieldAttributes, output::Output, root_attributes::RootAttributes, utils::{is_type}};

struct Wrapper {
    field: Field
}

impl Parse for Wrapper {
    fn parse(input: ParseStream) -> Result<Self> {
        let field = Field::parse_named(input)?;

        Ok(Self { field })
    }
}

pub fn create_location_field(field_name: &str) -> Field {
    let string = format!("pub {}: parsable::ItemLocation", field_name);
    let result : Result<Wrapper> = syn::parse_str(&string);

    result.unwrap().field
}

pub fn process_struct(data_struct: &mut DataStruct, root_attributes: &mut RootAttributes, output: &mut Output) {
    output.get_location = quote! {
        fn location(&self) -> &parsable::ItemLocation {
            &self.location
        }
    };

    let (root_markers_on_start, root_markers_on_exit, root_markers_on_fail) = root_attributes.get_push_pop_markers();
    let mut markers_on_fail = vec![root_markers_on_fail];

    match &mut data_struct.fields {
        Fields::Named(named_fields) => {
            let field_count = named_fields.named.len();
            let mut field_names = vec![];
            let mut lines = vec![];

            for (i, field) in named_fields.named.iter_mut().enumerate() {
                let attributes = FieldAttributes::from_field_attributes(&mut field.attrs);
                let (field_markers_on_start, field_markers_on_exit, field_markers_on_fail) = attributes.get_push_pop_markers(i);
                let is_vec = is_type(&field.ty, "Vec");
                let is_option = is_type(&field.ty, "Option");

                let field_name = field.ident.as_ref().unwrap();
                let field_type = &field.ty;

                field_names.push(quote! { #field_name });
                markers_on_fail.insert(0, field_markers_on_fail);

                let optional = is_option || attributes.optional.unwrap_or(false);
                let participate_in_cascade = root_attributes.cascade && attributes.cascade.unwrap_or(true);
                let consume_spaces = match attributes.consume_spaces {
                    Some(false) => quote! {},
                    _ => quote! { reader__.eat_spaces(); }
                };
                let mut handle_failure = quote! {};
                let mut on_fail = quote ! {
                    reader__.set_index(start_index__);
                    #(#markers_on_fail)*
                    #root_markers_on_exit
                    return None;
                };

                if optional {
                    let set_option_failed = match participate_in_cascade {
                        true => quote! { option_failed__ = true },
                        false => quote! {},
                    };

                    on_fail = quote! {
                        field_failed__ = true;
                        #set_option_failed;
                        reader__.set_index(field_index__);
                        <#field_type as Default>::default()
                    };

                    if attributes.suffix.is_some() {
                        handle_failure = quote! {
                            if field_failed__ {
                                #field_name = <#field_type as Default>::default();
                            }
                        }
                    }
                }

                let mut check = vec![];
                let has_prefix = attributes.prefix.is_some();
                let has_suffix = attributes.suffix.is_some();

                let mut pre_parsing_check = quote! {};

                if optional && participate_in_cascade {
                    pre_parsing_check = quote! {
                        if option_failed__ {
                            field_failed__ = true;
                        }
                    };
                }

                let prefix_parsing = match attributes.prefix {
                    Some(prefix) => {
                        let prefix_consume_spaces = match attributes.consume_spaces_after_prefix {
                            Some(false) => quote! { {} },
                            _ => quote! { reader__.eat_spaces() },
                        };

                        quote! {
                            if !field_failed__ {
                                match reader__.read_string(#prefix) {
                                    Some(_) => #prefix_consume_spaces,
                                    None => {
                                        reader__.set_expected_string(#prefix);
                                        prefix_ok__ = false;
                                        field_failed__ = true;
                                        #on_fail;
                                    }
                                };
                            }
                        }
                    },
                    None => quote! {}
                };
                let suffix_parsing = match attributes.suffix {
                    Some(suffix) => {
                        let suffix_consume_spaces = match attributes.consume_spaces_after_suffix {
                            Some(false) => quote! { {} },
                            _ => quote! { reader__.eat_spaces() },
                        };

                        quote! {
                            if !field_failed__ {
                                match reader__.read_string(#suffix) {
                                    Some(_) => #suffix_consume_spaces,
                                    None => {
                                        reader__.set_expected_string(#suffix);
                                        #on_fail;
                                    }
                                };
                            }
                        }
                    },
                    None => quote! {}
                };

                let mut exclude_parsing = quote! {};

                if let Some(exclude) = &attributes.exclude {
                    exclude_parsing = quote! {
                        if !field_failed__ && reader__.peek_regex(#exclude) {
                            #on_fail;
                        }
                    };
                }

                let mut followed_by_parsing = quote! {};

                if let Some(followed_by) = &attributes.followed_by {
                    followed_by_parsing = quote! {
                        if !field_failed__ && !reader__.peek_regex(#followed_by) {
                            reader__.set_expected_regex(#followed_by);
                            #on_fail;
                        }
                    };
                } else if let Some(not_followed_by) = &attributes.not_followed_by {
                    followed_by_parsing = quote! {
                        if !field_failed__ && reader__.peek_regex(#not_followed_by) {
                            // reader__.set_expected_regex(#not_followed_by);
                            #on_fail;
                        }
                    };
                }

                let mut parse_method = quote! { parse_item(reader__) };

                if is_vec {
                    if let Some(separator) = attributes.separator {
                        parse_method = quote! { parse_item_with_separator(reader__, #separator) };
                    } else if let Some(false) = attributes.consume_spaces_between_items {
                        parse_method = quote! { parse_item_without_consuming_spaces(reader__) };
                    }
                }

                let mut assignment = quote! {
                    let mut #field_name = match <#field_type as parsable::Parsable>::#parse_method {
                        Some(value) => value,
                        None => {
                            reader__.set_expected_item::<#field_type>();
                            #on_fail
                        }
                    };
                };

                if (has_prefix || participate_in_cascade) && optional {
                    assignment = quote! {
                        let mut #field_name = match prefix_ok__ && !option_failed__ {
                            true => match <#field_type as parsable::Parsable>::#parse_method {
                                Some(value) => value,
                                None => {
                                    reader__.set_expected_item::<#field_type>();
                                    #on_fail
                                }
                            },
                            false => <#field_type as Default>::default()
                        };
                    };

                    // assignment = quote! {
                    //     let #field_name = <#field_type as Default>::default();
                    // };
                }

                let make_field_from_string = match is_option {
                    true => quote! { Some(value) },
                    false => quote! { value },
                };

                if let Some(pattern) = attributes.regex {
                    assignment = quote! {
                        let #field_name = match reader__.read_regex(#pattern) {
                            Some(value) => match <String as std::str::FromStr>::from_str(value) {
                                Ok(value) => #make_field_from_string,
                                Err(_) => { #on_fail }
                            },
                            None => { #on_fail }
                        };
                    };
                } else if let Some(literal) = attributes.value {
                    assignment = quote! {
                        let #field_name = match reader__.read_string(#literal) {
                            Some(value) => match <String as std::str::FromStr>::from_str(value) {
                                Ok(value) => #make_field_from_string,
                                Err(_) => { #on_fail }
                            },
                            None => { #on_fail }
                        };
                    };

                    if field_count == 1 && root_attributes.token.is_none() {
                        root_attributes.token = Some(literal.clone());
                    }
                }

                if let Some(min) = attributes.min {
                    check.push(quote! {
                        if !field_failed__ && #field_name.len() < #min {
                            reader__.set_expected_item::<#field_type>();
                            #on_fail;
                        }
                    });
                }

                if is_option && has_prefix {
                    check.push(quote! {
                        if #field_name.is_none() {
                            #on_fail;
                        }
                    });
                }

                if is_vec && has_prefix && !has_suffix {
                    check.push(quote! {
                        if #field_name.is_empty() && prefix_ok__ {
                            reader__.set_expected_item::<#field_type>();
                            #on_fail;
                        }
                    });
                }

                if attributes.ignore {
                    lines.push(quote! {
                        let #field_name = <#field_type as Default>::default();
                    });
                } else {
                    lines.push(quote! {
                        #field_markers_on_start
                        field_failed__ = false;
                        prefix_ok__ = true;
                        field_index__ = reader__.get_index();
                        #pre_parsing_check
                        #prefix_parsing
                        #exclude_parsing
                        #assignment
                        #(#check)*
                        #consume_spaces
                        #suffix_parsing
                        #followed_by_parsing
                        #handle_failure
                        #field_markers_on_exit
                    });
                }
            }

            let mut set_location = quote! {};

            if root_attributes.located {
                field_names.push(quote! { location });
                named_fields.named.insert(0, create_location_field("location"));
                set_location = quote! { let location = reader__.get_item_location(start_index__); };
            }

            output.parse_item = quote! {
                fn parse_item(reader__: &mut parsable::StringReader) -> Option<Self> {
                    let start_index__ = reader__.get_index();
                    let mut field_index__ : usize = 0;
                    let mut field_failed__ = false;
                    let mut prefix_ok__ = true;
                    let mut option_failed__ = false;
                    #root_markers_on_start
                    #(#lines)*
                    #root_markers_on_exit
                    #set_location
                    Some(Self { #(#field_names),* })
                }
            };

        },
        Fields::Unnamed(_) => unreachable!(),
        Fields::Unit => unreachable!()
    }
}