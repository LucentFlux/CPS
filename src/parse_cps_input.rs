use crate::parse_macro_decl::{parse_brace, parse_paren};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::{Brace, Or, Paren};
use syn::Token;

pub const CPS_MARKER_STR: &'static str = "_cps";

#[derive(Clone)]
pub struct Divider {
    lhs: Token![|],
    mid: Token![:],
    rhs: Token![|],
}

impl Divider {
    fn peek(input: ParseStream) -> bool {
        return input.peek(Token![|]) && input.peek2(Token![:]) && input.peek3(Token![|]);
    }
}

impl Parse for Divider {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Divider {
            lhs: input.parse()?,
            mid: input.parse()?,
            rhs: input.parse()?,
        })
    }
}

#[derive(Clone)]
pub struct ParenthesizedIdent {
    paren: Paren,
    pub ident: Ident,
}

impl Parse for ParenthesizedIdent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (paren, ident) = parse_paren(input)?;
        Ok(Self {
            paren,
            ident: syn::parse2(ident)?,
        })
    }
}

impl ToTokens for ParenthesizedIdent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { ident, .. } = self;
        *tokens = quote!(
            #tokens (#ident)
        )
    }
}

#[derive(Clone)]
pub struct BracedTS {
    paren: Brace,
    pub internal: TokenStream,
}

impl Parse for BracedTS {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (paren, internal) = parse_brace(input)?;
        Ok(Self { paren, internal })
    }
}

impl ToTokens for BracedTS {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { internal, .. } = self;
        *tokens = quote!(
            #tokens {#internal}
        )
    }
}

#[derive(Clone)]
pub struct MacroInput {
    marker: Token![@],
    ident: Ident,
    div1: Divider,
    pub program: Punctuated<ParenthesizedIdent, Token![|]>,
    div2: Divider,
    pub stack: Vec<(Vec<BracedTS>, Token![|])>,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let res = MacroInput {
            marker: input.parse()?,
            ident: input.parse()?,
            div1: input.parse()?,
            program: {
                let mut program = Punctuated::new();
                if !Divider::peek(input) {
                    let first = input.parse()?;
                    program.push_value(first);

                    while !Divider::peek(input) {
                        program.push_punct(input.parse()?);

                        let next = input.parse()?;
                        program.push_value(next);
                    }
                }
                program
            },
            div2: input.parse()?,
            stack: {
                let mut stack = Vec::new();
                while !input.is_empty() {
                    let mut part = Vec::new();
                    while !input.peek(Token![|]) {
                        let arg = input.parse()?;
                        part.push(arg);
                    }
                    let sep: Or = input.parse()?;
                    stack.push((part, sep));
                }
                stack
            },
        };

        // Check assertions
        if !res.ident.to_string().eq(CPS_MARKER_STR) {
            return Err(syn::Error::new(
                input.span(),
                format!(
                    "incorrect cps macro marker: expected {} but got {}",
                    CPS_MARKER_STR,
                    res.ident.to_string()
                ),
            ));
        }

        return Ok(res);
    }
}
