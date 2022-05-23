use proc_macro2::{Span};
use syn::{Type, Ident};

pub fn is_type(ty: &Type, name: &str) -> bool {
    get_type_name(ty) == name
}

fn get_type_name(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => type_path.path.segments.last().unwrap().ident.to_string(),
        _ => todo!(),
    }
}

pub fn make_ident(name: String) -> Ident {
    Ident::new(&name, Span::call_site())
}