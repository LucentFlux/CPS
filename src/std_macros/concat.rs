use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, LitBool, Token};

enum Literal {
    Literal(proc_macro2::Literal),
    True,
    False,
}

impl Parse for Literal {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Ident::peek_any) {
            let ident: LitBool = input
                .parse()
                .map_err(|e| input.error(format!("{} when parsing token {}", e, input)))?;
            return match ident.value {
                true => Ok(Self::True),
                false => Ok(Self::False),
            };
        }
        Ok(Self::Literal(input.parse().map_err(|e| {
            input.error(format!("{} when parsing token {}", e, input))
        })?))
    }
}

impl ToString for Literal {
    fn to_string(&self) -> String {
        match self {
            Literal::Literal(l) => {
                match litrs::Literal::try_from(l).expect("literal was not a literal") {
                    litrs::Literal::String(s) => s.value().to_string(),
                    litrs::Literal::Char(s) => s.value().to_string(),
                    v => v.to_string(),
                }
            }
            Literal::True => "true".to_string(),
            Literal::False => "false".to_string(),
        }
    }
}

struct ConcatInput {
    pub args: Punctuated<Literal, Token![,]>,
}

impl Parse for ConcatInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            args: Punctuated::parse_terminated(input)?,
        })
    }
}

super::impl_std_cps!(
    use super::ConcatInput;

    fn concat(tokens: proc_macro2::TokenStream) -> proc_macro2::Literal {
        let err = format!(
            "arguments given were not comma separated: {}",
            tokens.to_string()
        );
        let parsed: ConcatInput = match syn::parse2(tokens) {
            Err(e) => panic!("error parsing concat input: {}, {}", err, e),
            Ok(v) => v,
        };

        let mut string = String::new();
        for token in parsed.args.into_iter() {
            string += &token.to_string();
        }
        proc_macro2::Literal::string(&string)
    }
);
