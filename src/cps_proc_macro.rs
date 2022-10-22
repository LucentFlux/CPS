use crate::cps_macro::build_next_step;
use crate::parse_cps_input::MacroInput;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::Token;

pub trait CPSProcMacro {
    type Input: Parse;
    type Output: ToTokens;

    fn step(inp: Self::Input) -> Self::Output;
}

pub fn perform_macro<M: CPSProcMacro>(item: TokenStream) -> TokenStream {
    let mut m: MacroInput = syn::parse2(item).expect("failed to parse CPS macro input");

    // Extract single arg
    let (args, _) = m.stack.remove(0);
    assert_eq!(args.len(), 1);
    let arg = args.first().expect("found no argument to CPS macro");
    let arg = syn::parse2(arg.internal.clone().into_token_stream())
        .expect("failed to parse CPS macro input item");

    // Evaluate
    let res = M::step(arg);

    // If the program is done, emit the result
    let next_call = match m.program.first() {
        None => {
            assert_eq!(
                m.stack.len(),
                0,
                "cps evaluation was done but stack wasn't emptied"
            );

            return res.to_token_stream();
        }
        Some(v) => v.ident.clone(),
    };

    let mut remaining_calls: Punctuated<_, Token![|]> = Punctuated::new();
    for i in m.program.into_iter().skip(1) {
        remaining_calls.push(i)
    }

    let mut next_stack = quote! {};
    for part in m.stack {
        for item in part.0 {
            next_stack = quote! {#next_stack #item}
        }
        next_stack = quote! {#next_stack |}
    }

    return build_next_step(next_call.clone(), remaining_calls, res, next_stack);
}
