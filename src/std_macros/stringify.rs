super::impl_std_cps!(
    fn stringify(tokens: proc_macro2::TokenStream) -> proc_macro2::Literal {
        proc_macro2::Literal::string(&tokens.to_string())
    }
);
