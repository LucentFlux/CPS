use std::fs;

use proc_macro2::Literal;
use quote::ToTokens;
use syn::spanned::Spanned;

fn do_include_str(tokens: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let span = tokens.span();

    let path: Literal = match syn::parse2(tokens) {
        Err(e) => panic!("error parsing include input: {}", e),
        Ok(v) => v,
    };

    let path = match litrs::Literal::from(path.clone()) {
        litrs::Literal::String(s) => s.value().to_string(),
        _ => panic!("path {} was not a string", path),
    };

    let contents = fs::read_to_string(&path).unwrap_or_else(|_| panic!("include file {path} was unreadable - note that paths must be relative to the project's manifest file"));

    syn::LitStr::new(&contents, span).into_token_stream()
}

super::impl_std_cps!(
    fn include_str(tokens: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        include_str::do_include_str(tokens)
    }
);
