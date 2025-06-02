use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Ident, LitBool, Token};

enum Literal {
    True,
    False,
    Other(proc_macro2::Literal),
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
        Ok(Self::Other(input.parse().map_err(|e| {
            input.error(format!("{} when parsing token {}", e, input))
        })?))
    }
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(lit) => match litrs::Literal::from(lit) {
                litrs::Literal::String(s) => s.value().fmt(f),
                litrs::Literal::Char(s) => s.value().fmt(f),
                v => v.fmt(f),
            },
            Self::True => write!(f, "true"),
            Self::False => write!(f, "false"),
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
        let err = format!("arguments given were not comma separated: {}", tokens);
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
