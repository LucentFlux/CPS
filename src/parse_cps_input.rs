use crate::parse_macro_decl::{parse_brace, parse_paren};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::{Brace, Or, Paren};
use syn::Token;

pub const CPS_MARKER_STR: &'static str = "_cps";

/// The 'divider' token set we use to separate the call stack from data stack. Corresponds to `|:|`.
#[derive(Clone)]
pub struct Divider {
    _lhs: Token![|],
    _mid: Token![:],
    _rhs: Token![|],
}

impl Divider {
    fn peek(input: ParseStream) -> bool {
        return input.peek(Token![|]) && input.peek2(Token![:]) && input.peek3(Token![|]);
    }
}

impl Parse for Divider {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Divider {
            _lhs: input.parse()?,
            _mid: input.parse()?,
            _rhs: input.parse()?,
        })
    }
}

/// An identifier enclosed in parenthesis `()`.
#[derive(Clone)]
pub struct ParenthesizedIdent {
    _paren: Paren,
    pub ident: Ident,
}

impl Parse for ParenthesizedIdent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (paren, ident) = parse_paren(input)?;
        Ok(Self {
            _paren: paren,
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

/// A token stream enclosed in braces `{}`.
#[derive(Clone)]
pub struct BracedTS {
    _paren: Brace,
    pub internal: TokenStream,
}

impl Parse for BracedTS {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (paren, internal) = parse_brace(input)?;
        Ok(Self {
            _paren: paren,
            internal,
        })
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

/// Comma separated identical expressions in parentheses `{..foo..}, {..foo..}`
#[derive(Clone)]
pub struct StackElementInner {
    pub lhs: BracedTS,
    _comma: Token![,],
    pub rhs: BracedTS,
}

impl Parse for StackElementInner {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lhs = BracedTS::parse(input)?;
        let comma = input.parse()?;
        let rhs = BracedTS::parse(input)?;

        Ok(Self {
            lhs,
            _comma: comma,
            rhs,
        })
    }
}

impl ToTokens for StackElementInner {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { lhs, rhs, .. } = self;
        *tokens = quote!(
            #tokens #lhs, #rhs
        )
    }
}

/// An element on the stack: two identical expressions in parentheses `({..foo..}, {..foo..})`
#[derive(Clone)]
pub struct StackElement {
    _paren: Paren,
    pub lhs: BracedTS,
    _comma: Token![,],
    pub rhs: BracedTS,
}

impl Parse for StackElement {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (paren, internal) = parse_paren(input)?;

        let StackElementInner { lhs, _comma, rhs } = syn::parse2(internal)?;

        Ok(Self {
            _paren: paren,
            lhs,
            _comma,
            rhs,
        })
    }
}

impl ToTokens for StackElement {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { lhs, rhs, .. } = self;
        *tokens = quote!(
            #tokens (#lhs, #rhs)
        )
    }
}

#[derive(Clone)]
pub struct MacroInput {
    _marker: Token![@],
    ident: Ident,
    _div1: Divider,
    pub program: Punctuated<ParenthesizedIdent, Token![|]>,
    _div2: Divider,
    pub stack: Vec<(Vec<StackElement>, Token![|])>,
}

impl Parse for MacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let res = MacroInput {
            _marker: input.parse()?,
            ident: input.parse()?,
            _div1: input.parse()?,
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
            _div2: input.parse()?,
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
