use crate::parse_macro_decl::{begins_with_cps_marker, CPSMacroRule, MacroMatch};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
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
            { #impl_tokens } #next_stack
        }
    }
}

fn add_cps(macro_name: &Ident, arm: CPSMacroRule) -> Vec<CPSMacroRule> {
    let pattern = arm.pattern.clone();
    let impl_tokens = arm.impl_tokens;

    let mut output = Vec::new();

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
    let base_case: CPSMacroRule = syn::parse2(quote! {
        (@_cps |:|  |:| #( { #result_patterns } )* { #pattern } | ) => {
            #impl_tokens
        }
    })
    .expect("could not build cps base case");
    output.push(base_case);

    // Above is a special case of this for reduced macro recursion depth
    let next_step = build_next_step(
        quote! { $_cps_next_head },
        quote! { $( ( $_cps_next_tail ) )|* },
        impl_tokens,
        quote! { $($_cps_stack)* },
    );
    let inner_base_case: CPSMacroRule = syn::parse2(quote!{
        (@_cps |:| ( $_cps_next_head:tt ) $(| ( $_cps_next_tail:tt ) )* |:| #( { #result_patterns } )* { #pattern } | $($_cps_stack:tt)*) => {
            #next_step
        }
    }).expect("could not build cps inner base case");
    output.push(inner_base_case);

    // Create a pattern for each intermediate step as earlier results may be used in later executions
    let mut acc_result_patterns = Vec::new();
    let mut acc_result_clones = Vec::new();
    let pattern_output_clone = pattern.build_output_clone();
    for binding in arm.let_bindings.iter() {
        let path_indirection = binding.macro_name_indirection.clone();
        let binding_macro_path = binding.macro_invocation.path.clone();
        let binding_macro_args = binding.macro_invocation.tokens.clone();
        let inter_case: CPSMacroRule = syn::parse2(quote!{
            (@_cps |:| $( ( $_cps_next:tt ) )|* |:| #( { #acc_result_patterns } )* { #pattern } | $($_cps_stack:tt)*) => {
                #path_indirection #binding_macro_path ! { @_cps |:|
                    ( #macro_name ) $(| ( $_cps_next:path ) )* |:|
                    { #binding_macro_args } | #( { #acc_result_clones } )* { #pattern_output_clone } | $($_cps_stack)*
                }
            }
        }).expect("could not build cps inter case");
        output.push(inter_case);

        let result_pattern = result_patterns
            .pop()
            .expect("different number of matches to let bindings");
        acc_result_clones.insert(0, result_pattern.build_output_clone());
        acc_result_patterns.insert(0, result_pattern);
    }

    // Create an entry point from outside a cps context
    let pattern_out = pattern.build_output_clone();
    let entry: CPSMacroRule = syn::parse2(quote! {
        (#pattern) => {
            #macro_name ! { @_cps |:|  |:| { #pattern_out } | }
        }
    })
    .expect("could not build cps entry case");
    output.push(entry);

    return output;
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
    for rule in rules {
        let mut new_cps_rules = add_cps(&macro_name, rule);
        new_rules.append(&mut new_cps_rules);
    }

    // Rebuild macro
    let attrs = m.attrs;
    let path = m.mac.path;
    let semi = m.semi_token;
    let rebuilt = quote! {
        #(#attrs)*
        #path ! #macro_name {
            #(#new_rules);*
        } #semi
    };

    rebuilt
}

/*
#[cps]
macro_rules! example {
    (...) => {
        ...
    }
}

should become

macro_rules! example {
    (@_cps |:|  |:| { ... }) => {
        ...
    };

    (@_cps
    |:|
        $_cps_cont:ident !
        $(
            ; $_cps_cont_rest:ident !
        )*
    |:| { ... } $({$($_cps_stack:tt)*})*) => {
        $_cps_cont ! {@_cps
            |:|
                $(
                    $_cps_cont_rest:ident !
                );*
            |:| { ... } $({$($_cps_stack)*})*
        }
    };

    (...) => {
        example! (@_cps |:|  |:| { ... })
    };
}



#[cps]
macro_rules! example {
    (a, ...) => {
        ...
        example!(b, ...)
        ...
    };

    (b, ...) => {
        ...
    }
}

should become

macro_rules! example {
    (@_cps |:|  |:| { a, ... }) => {
        ...
    };

    (@_cps |:|  |:| { b, ... }) => {
        ...
    };

    (@_cps
    |:|
        $_cps_cont:ident !
        $(
            ; $_cps_cont_rest:ident !
        )*
    |:| { a, ... } $({$($_cps_stack:tt)*})*) => {
        $_cps_cont ! {@_cps
            |:|
                $(
                    $_cps_cont_rest:ident !
                );*
            |:| { ... } $({$($_cps_stack)*})*
        }
    };

    (@_cps
    |:|
        $_cps_cont:ident !
        $(
            ; $_cps_cont_rest:ident !
        )*
    |:| { b, ... } $({$($_cps_stack:tt)*})*) => {
        $_cps_cont ! {@_cps
            |:|
                $(
                    $_cps_cont_rest:ident !
                );*
            |:| { ... } $({$($_cps_stack)*})*
        }
    };

    (...) => {
        example! (@_cps |:|  |:| { ... })
    };
}




#[cps]
macro_rules! example {
    (a, ...) =>
    let $b1 = example!(b, ...) in
    let $b2 = example!(b, ..., {$b1}, ...) in
    {
        ...
        $b1
        ...
        $b2
        ...
    };

    (b, ...) => {
        ...
    }
}

should become

macro_rules! example {
    (@_cps |:|  |:| { $($b2:tt)* } { $($b1:tt)* } { a, ... }) => {
        ...
        $($b1)*
        ...
        $($b2)*
        ...
    };

    (@_cps |:|  |:| { b, ... }) => {
        ...
    };

    (@_cps
    |:|
        $(
            $_cps_conts:ident !
        );*
    |:|
        { a, ... } | $($_cps_stack:tt)*
    ) => {
        example! {@_cps
            |:|
            example! ; example! ; $( $_cps_conts ! );*
            |:|
            { b, ... } | push_1 { b, ..., _, ... } | $($_cps_stack)*
        }
    };

    (@_cps
    |:|
        $_cps_cont:ident !
        $(
            ; $_cps_cont_rest:ident !
        )*
    |:|
        { $($b1:tt)* } push_1 { b, ..., _, ... } | $($_cps_stack:tt)*
    ) => {
        $_cps_cont {@_cps
            |:|
                $(
                    $_cps_cont_rest:ident !
                );*
            |:|
            { b, ..., { $($b1)* }, ... } | { $($b1)* } $($_cps_stack)*
        }
    };

    (@_cps
    |:|
        $_cps_cont:ident !
        $(
            ; $_cps_cont_rest:ident !
        )*
    |:| { b, ... } | $($_cps_stack:tt)*) => {
        $_cps_cont ! {@_cps
            |:|
                $(
                    $_cps_cont_rest:ident !
                );*
            |:| { ... } $($_cps_stack)*
        }
    };

    (...) => {
        example! (@_cps |:|  |:| { ... })
    };
}
*/
