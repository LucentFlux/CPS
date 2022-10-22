use proc_macro2::{Delimiter, Literal, Punct, TokenStream, TokenTree};
use quote::{quote, ToTokens, TokenStreamExt};
use std::fmt::Display;
use syn::ext::IdentExt;
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::token::{Brace, Paren, Token};
use syn::{parse2, Ident, Macro, MacroDelimiter, Token};

trait MyExtendable {
    fn add_message<S: Display>(self, input: ParseStream, msg: S) -> Self;
}

impl<T> MyExtendable for syn::Result<T> {
    fn add_message<S: Display>(self, input: ParseStream, msg: S) -> Self {
        self.map_err(|e| input.error(format!("{}: {}", msg, e.to_string())))
    }
}

fn parse_delimiter(input: ParseStream) -> syn::Result<(MacroDelimiter, TokenStream)> {
    input.step(|cursor| {
        let err = cursor.error("expected delimiter");
        if let Some((TokenTree::Group(g), rest)) = cursor.token_tree() {
            let span = g.span();
            let delimiter = match g.delimiter() {
                Delimiter::Parenthesis => MacroDelimiter::Paren(syn::token::Paren(span)),
                Delimiter::Brace => MacroDelimiter::Brace(syn::token::Brace(span)),
                Delimiter::Bracket => MacroDelimiter::Bracket(syn::token::Bracket(span)),
                Delimiter::None => {
                    return Err(err);
                }
            };
            Ok(((delimiter, TokenStream::from(g.stream())), rest))
        } else {
            Err(err)
        }
    })
}

fn macro_delimiter_to_tokens<T: ToTokens>(delimiter: &MacroDelimiter, internal: &T) -> TokenStream {
    match delimiter {
        MacroDelimiter::Paren(_) => quote! { ( #internal ) },
        MacroDelimiter::Brace(_) => quote! { { #internal } },
        MacroDelimiter::Bracket(_) => quote! { [ #internal ] },
    }
}

fn delimiter_to_tokens<T: ToTokens>(delimiter: &Delimiter, internal: &T) -> TokenStream {
    match delimiter {
        Delimiter::Parenthesis => quote! { ( #internal ) },
        Delimiter::Brace => quote! { { #internal } },
        Delimiter::Bracket => quote! { [ #internal ] },
        Delimiter::None => quote! { #internal },
    }
}

pub fn parse_paren(input: ParseStream) -> syn::Result<(Paren, TokenStream)> {
    let err = input.error("expected parenthesis");
    let (delim, ts) = match parse_delimiter(input) {
        Ok(v) => v,
        _ => return Err(err),
    };
    let paren = match delim {
        MacroDelimiter::Paren(p) => p,
        _ => return Err(err),
    };
    return Ok((paren, ts));
}

pub fn parse_brace(input: ParseStream) -> syn::Result<(Brace, TokenStream)> {
    let err = input.error("expected brace");
    let (delim, ts) = match parse_delimiter(input) {
        Ok(v) => v,
        _ => return Err(err),
    };
    let brace = match delim {
        MacroDelimiter::Brace(p) => p,
        _ => return Err(err),
    };
    return Ok((brace, ts));
}

pub fn begins_with_cps_marker(item: &MacroMatcher) -> bool {
    if let Some(MacroMatch::Punct(p)) = item.matches.get(0) {
        if p.as_char() == '@' {
            if let Some(MacroMatch::Ident(i)) = item.matches.get(0) {
                if i.to_string() == "_cps" {
                    return true;
                }
            }
        }
    }

    return false;
}

#[derive(Debug, Clone)]
pub struct MacroRepSep {
    pub token: TokenTree,
}

impl Parse for MacroRepSep {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.step(|cursor| {
            if let Some((token, rest)) = cursor.token_tree() {
                match token {
                    TokenTree::Group(_) => Err(cursor.error("did not expect delimiter")),
                    _ => Ok((Self { token }, rest)),
                }
            } else {
                Err(cursor.error("expected token"))
            }
        })
    }
}

impl ToTokens for MacroRepSep {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { token } = self.clone();

        tokens.append(token)
    }
}

#[derive(Clone)]
pub struct MacroVariableIdentifier {
    pub dollar_sign: Token![$],
    pub identifier: Ident,
    pub colon: Token![:],
    pub macro_frag_spec: Ident,
}

impl MacroVariableIdentifier {
    pub fn build_output_clone(&self) -> TokenStream {
        let Self {
            dollar_sign,
            identifier,
            ..
        } = self;

        return quote! {
            #dollar_sign #identifier
        };
    }

    fn parse_helper(input: ParseStream) -> syn::Result<Self> {
        let dollar_sign = input.parse()?;
        let identifier = input.parse()?;
        let colon = input.parse()?;
        let macro_frag_spec = input.parse()?;
        return Ok(Self {
            dollar_sign,
            identifier,
            colon,
            macro_frag_spec,
        });
    }
}

impl Parse for MacroVariableIdentifier {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Self::parse_helper(input).add_message(input, "could not parse macro variable identifier")
    }
}

impl ToTokens for MacroVariableIdentifier {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            dollar_sign,
            identifier,
            colon,
            macro_frag_spec,
        } = self.clone();
        *tokens = quote! {
            #tokens

            #dollar_sign
            #identifier
            #colon
            #macro_frag_spec
        };
    }
}

#[derive(Clone)]
pub enum MacroRepOp {
    Times(Token![*]),
    Plus(Token![+]),
    Optional(Token![?]),
}

impl Parse for MacroRepOp {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(Token![*]) {
            return Ok(Self::Times(input.parse()?));
        }
        if input.peek(Token![+]) {
            return Ok(Self::Plus(input.parse()?));
        }
        if input.peek(Token![?]) {
            return Ok(Self::Optional(input.parse()?));
        }
        return Err(input.error("expected a repetition operator"));
    }
}

impl ToTokens for MacroRepOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            MacroRepOp::Times(t) => t.to_tokens(tokens),
            MacroRepOp::Plus(p) => p.to_tokens(tokens),
            MacroRepOp::Optional(o) => o.to_tokens(tokens),
        }
    }
}

#[derive(Clone)]
pub struct MacroRepetition {
    pub dollar_sign: Token![$],
    pub paren: Paren,
    pub sub_matches: MacroMatcher,
    pub rep_sep: Option<MacroRepSep>,
    pub rep_op: MacroRepOp,
}

impl MacroRepetition {
    pub fn build_output_clone(&self) -> TokenStream {
        let Self {
            dollar_sign,
            sub_matches,
            rep_sep,
            ..
        } = self;

        let sub_matches = sub_matches.build_output_clone();

        return quote! {
            #dollar_sign ( #sub_matches ) #rep_sep *
        };
    }

    fn parse_helper(input: ParseStream) -> syn::Result<Self> {
        let dollar_sign = input.parse()?;
        let (paren, rep_pattern) = parse_paren(input)?;
        let sub_matches = syn::parse2(rep_pattern)?;
        let rep_sep = if !input.peek(Token![*]) && !input.peek(Token![+]) && !input.peek(Token![?])
        {
            Some(input.parse()?)
        } else {
            None
        };
        let rep_op = input.parse()?;

        Ok(Self {
            dollar_sign,
            paren,
            sub_matches,
            rep_sep,
            rep_op,
        })
    }
}

impl Parse for MacroRepetition {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Self::parse_helper(input).add_message(input, "could not parse macro repetition")
    }
}

impl ToTokens for MacroRepetition {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            dollar_sign,
            sub_matches,
            rep_sep,
            rep_op,
            ..
        } = self.clone();
        *tokens = quote! {
            #tokens

            #dollar_sign
            (
                #sub_matches
            )
            #rep_sep
            #rep_op
        }
    }
}

#[derive(Clone)]
pub enum MacroMatch {
    Ident(Ident),
    Punct(Punct),
    Literal(Literal),
    Group(Delimiter, MacroMatcher),
    Identifier(MacroVariableIdentifier),
    Repetition(MacroRepetition),
}

impl MacroMatch {
    pub fn build_output_clone(&self) -> TokenStream {
        match self {
            MacroMatch::Ident(t) => quote! {#t},
            MacroMatch::Punct(t) => quote! {#t},
            MacroMatch::Literal(t) => quote! {#t},
            MacroMatch::Group(d, t) => {
                let oc = t.build_output_clone();
                delimiter_to_tokens(&d, &oc)
            }
            MacroMatch::Identifier(i) => i.build_output_clone(),
            MacroMatch::Repetition(r) => r.build_output_clone(),
        }
    }
}

fn parse_ident_or_recursive(input_fork: &ParseBuffer) -> Option<MacroMatch> {
    if input_fork.peek(Token![$]) {
        if input_fork.peek2(Ident::peek_any) {
            // Identifier
            return input_fork.parse().map(|i| MacroMatch::Identifier(i)).ok();
        } else if input_fork.peek2(Paren) {
            // Repetition
            return input_fork.parse().map(|i| MacroMatch::Repetition(i)).ok();
        }
    }

    return None;
}

impl Parse for MacroMatch {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let input_fork = input.fork();
        return match parse_ident_or_recursive(&input_fork) {
            Some(m) => {
                input.advance_to(&input_fork);
                Ok(m)
            }
            None => {
                // On fail to parse, assume Token
                let tt = input.parse::<TokenTree>()?;
                match tt {
                    TokenTree::Ident(i) => Ok(Self::Ident(i)),
                    TokenTree::Punct(p) => Ok(Self::Punct(p)),
                    TokenTree::Literal(l) => Ok(Self::Literal(l)),
                    TokenTree::Group(g) => {
                        let delim = g.delimiter();
                        let internal = g.stream();

                        let matcher = parse2::<MacroMatcher>(internal)?;

                        Ok(Self::Group(delim, matcher))
                    }
                }
            }
        };
    }
}

impl ToTokens for MacroMatch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            MacroMatch::Ident(t) => t.to_tokens(tokens),
            MacroMatch::Punct(t) => t.to_tokens(tokens),
            MacroMatch::Literal(t) => t.to_tokens(tokens),
            MacroMatch::Group(d, t) => delimiter_to_tokens(&d, &t).to_tokens(tokens),
            MacroMatch::Identifier(i) => i.to_tokens(tokens),
            MacroMatch::Repetition(r) => r.to_tokens(tokens),
        }
    }
}

#[derive(Clone)]
pub struct MacroMatcher {
    pub matches: Vec<MacroMatch>,
}

impl MacroMatcher {
    pub fn build_output_clone(&self) -> TokenStream {
        let mapped = self
            .matches
            .iter()
            .map(|m| m.build_output_clone())
            .collect::<Vec<TokenStream>>();
        quote! {
            #(#mapped)*
        }
    }
}

impl Parse for MacroMatcher {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut matches = Vec::new();
        while !input.is_empty() {
            let new_match = input.parse::<MacroMatch>()?;
            matches.push(new_match)
        }

        return Ok(Self { matches });
    }
}

impl ToTokens for MacroMatcher {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let self_matches = self.matches.clone();
        *tokens = quote! {
            #tokens

            #(#self_matches)*
        };
    }
}

#[derive(Clone)]
pub struct LetBinding {
    pub let_token: Token![let],
    pub pattern: MacroMatch, // Single macro match, have to use braces for more interesting matches
    pub equals_token: Token![=],
    pub macro_name_indirection: Option<Token![$]>,
    pub macro_invocation: Macro,
    pub in_token: Token![in],
}

impl LetBinding {
    fn parse_helper(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            let_token: input.parse()?,
            pattern: input.parse()?,
            equals_token: input.parse()?,
            macro_name_indirection: input.parse()?,
            macro_invocation: input.parse()?,
            in_token: input.parse()?,
        })
    }
}

impl Parse for LetBinding {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Self::parse_helper(input).add_message(input, "could not parse cps macro let binding")
    }
}

#[derive(Clone)]
pub struct CPSMacroRule {
    pub pattern_brace: MacroDelimiter,
    pub pattern: MacroMatcher,
    pub let_bindings: Vec<LetBinding>,
    pub fat_arrow: Token![=>],
    pub impl_brace: MacroDelimiter,
    pub impl_tokens: TokenStream,
}

impl Parse for CPSMacroRule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let (pattern_brace, pattern) = parse_delimiter(input)?;

        let pattern = syn::parse2(pattern)?;

        let fat_arrow = input.parse::<Token![=>]>()?;

        let mut let_bindings = Vec::new();
        while input.peek(Token![let]) {
            let let_expr = LetBinding::parse(input)?;
            let_bindings.push(let_expr);
        }

        let (impl_brace, impl_tokens) = parse_delimiter(input)?;

        return Ok(Self {
            pattern_brace,
            pattern,
            let_bindings,
            fat_arrow,
            impl_brace,
            impl_tokens,
        });
    }
}

impl ToTokens for CPSMacroRule {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let CPSMacroRule {
            pattern_brace,
            pattern,
            let_bindings,
            fat_arrow,
            impl_brace,
            impl_tokens,
        } = self.clone();

        assert!(let_bindings.is_empty()); // Don't bother rendering let bindings because we shouldn't need to

        let braced_pattern = macro_delimiter_to_tokens(&pattern_brace, &pattern);
        let braced_impl = macro_delimiter_to_tokens(&impl_brace, &impl_tokens);

        *tokens = quote! {
            #tokens

            #braced_pattern
            #fat_arrow
            #braced_impl
        };
    }
}
