use std::collections::HashMap;

use crate::parse_macro_decl::{begins_with_cps_marker, CPSMacroRule, MacroMatch, MacroMatcher};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens, format_ident};
use syn::punctuated::Punctuated;
use syn::{parse_quote, ItemMacro, Token};

fn assert_arm_valid(m: &CPSMacroRule) {
    // Check that initial pattern is not the cps identifier
    assert!(
        !begins_with_cps_marker(&m.pattern),
        "macro rules cannot begin with @_cps when using the cps attribute"
    );
}

pub fn build_next_step(
    next_head: impl ToTokens,
    next_program: impl ToTokens,
    impl_tokens: impl ToTokens,
    next_stack: impl ToTokens,
) -> TokenStream {
    quote! {
         #next_head ! { @_cps |:|
            #next_program |:|
            ({ #impl_tokens }, { #impl_tokens }) #next_stack
        }
    }
}

fn add_cps(
    macro_name: &Ident,
    arm: CPSMacroRule,
) -> (
    Vec<CPSMacroRule>,
    HashMap<String, (MacroMatcher, Vec<MacroMatcher>)>,
) {
    let pattern = arm.pattern.clone();
    let impl_tokens = arm.impl_tokens;

    let mut output_cases = Vec::new();
    let mut output_debug_cases = HashMap::new();

    // Functions can be evaluated in several contexts:
    // 1. Base Case - they are the last function to execute and all of their bindings have been evaluated
    // 2. Inner Base Case - all of their bindings have been evaluated but there is more to do
    // 3+. Intermediate Case - they are entered with a partial stack and have to evaluate more of their bindings

    // Create a pattern for the base cps case where the arguments have been evaluated
    let mut result_patterns = arm
        .let_bindings
        .iter()
        .rev()
        .map(|lb| lb.pattern.clone())
        .collect::<Vec<MacroMatch>>();
    let result_pattern_duds: Vec<_> = result_patterns.iter().enumerate().map(|(i, _)| format_ident!("_cps_res{}", i)).collect();
    let base_case: CPSMacroRule = syn::parse2(quote! {
        (@_cps |:|  |:| #( ({ #result_patterns }, { $($ #result_pattern_duds :tt)* }) )* ({ #pattern }, { $($_cps_dud_pattern:tt)* }) | ) => {
            #impl_tokens
        }
    })
    .expect("could not build cps base case");
    output_cases.push(base_case);

    // Above is a special case of this for reduced macro recursion depth
    let next_step = build_next_step(
        quote! { $_cps_next_head },
        quote! { $( ( $_cps_next_tail ) )|* },
        impl_tokens,
        quote! { $($_cps_stack)* },
    );
    let inner_base_case: CPSMacroRule = syn::parse2(quote!{
        (@_cps |:| ( $_cps_next_head:tt ) $(| ( $_cps_next_tail:tt ) )* |:| #( ({ #result_patterns }, { $($ #result_pattern_duds :tt)* }) )* ({ #pattern }, { $($_p:tt)* }) | $($_cps_stack:tt)*) => {
            #next_step
        }
    }).expect("could not build cps inner base case");
    output_cases.push(inner_base_case);

    // Create a pattern for each intermediate step as earlier results may be used in later executions
    let mut acc_result_patterns = Vec::new();
    let mut acc_result_tts = Vec::new();
    let mut acc_result_clones = Vec::new();
    for (i, binding) in arm.let_bindings.iter().enumerate() {
        let path_indirection = binding.macro_name_indirection.clone();
        let binding_macro_path = binding.macro_invocation.path.clone();
        let binding_macro_args = binding.macro_invocation.tokens.clone();
        // Successful inner case
        let inter_case: CPSMacroRule = syn::parse2(quote!{
            (@_cps |:| $( ( $_cps_next:tt ) )|* |:| #( ({ #acc_result_patterns }, { #acc_result_tts }) )* ({ #pattern }, {$($_cps_arg:tt)*}) | $($_cps_stack:tt)*) => {
                #path_indirection #binding_macro_path ! { @_cps |:|
                    ( #macro_name ) $(| ( $_cps_next ) )* |:|
                    ({ #binding_macro_args }, { #binding_macro_args }) | #( ({ #acc_result_clones }, { #acc_result_clones }) )* ({$($_cps_arg)*}, {$($_cps_arg)*}) | $($_cps_stack)*
                }
            }
        }).expect("could not build cps inter case");
        output_cases.push(inter_case);

        // Unsuccessful pattern match on inner case
        let mut valid_patterns = acc_result_patterns
            .clone()
            .into_iter()
            .map(|m: MacroMatch| MacroMatcher {
                matches: vec![m.clone()],
            })
            .collect::<Vec<_>>();
        valid_patterns.push(pattern.clone());
        let valid_pattern_duds: Vec<_> = valid_patterns.iter().enumerate().map(|(i, _)| format_ident!("_cps_res{}", i)).collect();
        let expected_pattern = valid_patterns.remove(0);
        let invalid_match: MacroMatcher = syn::parse2(quote!{
            @_cps |:| $( ( $_cps_next:tt ) )|* |:| ({ $($unexpected:tt)* }, { $($_cps_un2:tt)* }) #( ({ #valid_patterns }, { $($ #valid_pattern_duds :tt)* }) )* | $($_cps_stack:tt)*
        }).expect("could not build cps inter debug match");
        output_debug_cases
            .entry(invalid_match.to_token_stream().to_string())
            .or_insert((invalid_match, vec![]))
            .1
            .push(expected_pattern);

        let result_pattern = result_patterns
            .pop()
            .expect("different number of matches to let bindings");

        let tt_ident = format_ident!("_cps_arg{}", i);
        acc_result_tts.insert(0, quote!{$($ #tt_ident :tt)*});
        acc_result_clones.insert(0, quote!{$($ #tt_ident)*});
        acc_result_patterns.insert(0, result_pattern);
    }

    return (output_cases, output_debug_cases);
}

pub fn impl_cps(_attr: TokenStream, m: ItemMacro) -> TokenStream {
    // Check we're being applied to a macro_rules! definition
    let err = "expected a macro_rules! macro definition";
    assert_eq!(
        "macro_rules",
        m.mac.path.segments.last().expect(err).ident.to_string(),
        "{}",
        err
    );
    let macro_name = m.ident.expect(err);

    // Parse rules
    let rules_tokens = TokenStream::from(m.mac.tokens);
    let rules: Punctuated<CPSMacroRule, Token![;]> = parse_quote! { #rules_tokens };

    // Check that all rules are of valid form
    for rule in rules.iter() {
        assert_arm_valid(rule);
    }

    // Add cps to all rules
    let mut new_rules = Vec::new();
    let mut error_rules = HashMap::new();
    for rule in rules {
        let (mut new_cps_rules, new_error_rules) = add_cps(&macro_name, rule);
        new_rules.append(&mut new_cps_rules);

        for (s, (error_match, mut expected_patterns)) in new_error_rules {
            error_rules.entry(s).or_insert((error_match, vec![])).1.append(&mut expected_patterns);
        }
    }

    // Collate same errors into messages
    let error_rules = error_rules.into_iter().map(|(_, (error_match, expected_patterns))| {
        let err_msg = if expected_patterns.len() == 1 {
            let expected_pattern = expected_patterns.first().expect("len is 1");
            format!(
                "while evaluating macro {}, expected something that matches `{}` but got `",
                macro_name.to_string(),
                expected_pattern.to_token_stream()
            )
        } else {
            let parts = 
            expected_patterns.iter().map(|expected_pattern| format!("`{}`", expected_pattern.to_token_stream())).fold("".to_owned(), |a, b| a + " or " + &b);
            format!(
                "while evaluating macro {}, expected something that matches one of {} but got `",
                macro_name.to_string(),
                parts
            )
        };

        syn::parse2::<CPSMacroRule>(quote!{
            (#error_match) => {
                std::compile_error!(std::concat!(#err_msg, std::stringify!($($unexpected)*) ,"` instead"));
            }
        }).expect("could not build cps inter debug case")
    }).collect::<Vec<_>>();

    // Add some fallback CPS rules that can help with debugging
    let fallback_rules = quote! {
        // If nothing else matches
        (@_cps |:| $(($call_stack:tt))|* |:| ({ $($unexpected:tt)* }, { $($_un2:tt)* }) $($data_stack:tt)* ) => {
            std::compile_error!(concat!("cannot match `", stringify!($($unexpected)*), "`"));
        };
        // Base case but wrong arguments
        (@_cps |:|  |:|  |) => {
            std::compile_error!("base case has no result - this is a bug with the cps crate and should be reported here: https://github.com/LucentFlux/CPS/issues");
        };
        (@_cps $($everything:tt)*) => {
            std::compile_error!(concat!("cps macro evaluation resulted in an invalid state: `", stringify!($($everything)*), "` - this is a bug with the cps crate and should be reported here: https://github.com/LucentFlux/CPS/issues"));
        };
    };

    // Create an entry point from outside a cps context
    let entry = quote! {
        ($($input:tt)*) => {
            #macro_name ! { @_cps |:|  |:| ({ $($input)* }, { $($input)* }) | }
        };
    };

    // Rebuild macro
    let attrs = m.attrs;
    let path = m.mac.path;
    let semi = m.semi_token;
    let rebuilt = quote! {
        #(#attrs)*
        #path ! #macro_name {
            #(#new_rules ;)*
            #(#error_rules ;)*
            #fallback_rules
            #entry
        } #semi
    };

    rebuilt
}
