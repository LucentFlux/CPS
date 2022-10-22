#![deny(missing_docs)]

/*!
Macro execution order is tricky. For example, the output of the following code goes against our
intuition of how functions should work:

```
macro_rules! expand_to_larch {
    () => { larch };
}

macro_rules! recognize_tree {
    (larch) => { println!("#1, the Larch.") };
    (redwood) => { println!("#2, the Mighty Redwood.") };
    (fir) => { println!("#3, the Fir.") };
    (chestnut) => { println!("#4, the Horse Chestnut.") };
    (pine) => { println!("#5, the Scots Pine.") };
    ($($other:tt)*) => { println!("I don't know; some kind of birch maybe?") };
}

fn main() {
    recognize_tree!(expand_to_larch!()); // Prints "I don't know; some kind of birch maybe?"
}
```

[The Little Book of Rust Macros][tlborm] (where the above example comes from) outlines *callbacks* -
a macro pattern that allows macro execution order to be specified:

```
# macro_rules! recognize_tree {
#    (larch) => { println!("#1, the Larch.") };
#    (redwood) => { println!("#2, the Mighty Redwood.") };
#    (fir) => { println!("#3, the Fir.") };
#    (chestnut) => { println!("#4, the Horse Chestnut.") };
#    (pine) => { println!("#5, the Scots Pine.") };
#    ($($other:tt)*) => { println!("I don't know; some kind of birch maybe?") };
# }

macro_rules! call_with_larch {
    ($callback:ident) => { $callback!(larch) };
}

fn main() {
    call_with_larch!(recognize_tree); // Correctly prints "#1, the Larch."
}
```

This syntax, while powerful, soon becomes confusing.

This macro allows far more readable macros to be written:

```
# use cps::cps;

#[cps]
macro_rules! expand_to_larch {
    () => { larch };
}

#[cps]
macro_rules! recognize_tree {
    (larch) => { println!("#1, the Larch.") };
    // ...
    ($($other:tt)*) => { println!("I don't know; some kind of birch maybe?") };
}

#[cps]
macro_rules! name_a_larch {
    () =>
    let $tree:tt = expand_to_larch!() in
    {
        recognize_tree!($tree)
    };
}

fn main() {
    name_a_larch!(); // Prints "#1, the Larch."
}
```

Macros-by-example are hard, difficult to maintain, and you should always consider writing a proc-macro instead.
This library aims to make the macros that you *do* write more maintainable. Please recurse responsibly.

## Usage Notes

CPS converts iteration into recursion. Therefore when using this library you may reach the recursion limit (128 at the time of writing). You can raise this using `#![recursion_limit = "1024"]` but your build times may suffer.

Any macro `let` expression must have a macro on the right-hand side that was marked as `#[cps]`. The following example will not work:

```
# use cps::cps;
#[cps]
macro_rules! foo {
    () => { BaseCase };

    (bar) =>
    let $x:tt = foo!() in
    let $y:tt = stringify!($x) in // Issue: stringify is not a cps macro
    {
        $y
    };
}
```

Instead, use the `cps` variants of builtin macros:

```
# use cps::cps;
#[cps]
macro_rules! foo {
    () => { BaseCase };

    (bar) =>
    let $x:tt = foo!() in
    let $y:tt = cps::stringify!($x) in // cps::stringify is a cps version of `stringify`
    {
        $y
    };
}
```

 */

mod cps_macro;
mod cps_proc_macro;
mod parse_cps_input;
mod parse_macro_decl;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Ident, ItemMacro, LitBool, Token};

/// Manipulates a macro_rules! definition to add extended syntax to help in creating readable macros.
///
/// # Usage
///
/// CPS macros are a strict superset of Rust macro-rules. This means that any (*) regular rust macro can
/// be prefaced by `#[cps]` and behave exactly the same.
///
/// The added syntax is the new `let` bindings allowed before the body of macro rules. These let statements
/// allow other `cps` macros to be evaluated *before* the body is evaluated. Let bindings are executed in order,
/// and can refer to the results of previous let binding results.
///
/// \* You may not begin a rule with the tokens `@_cps`
///
/// ## Evaluation Order
///
/// ```
/// # use cps::cps;
/// #[cps]
/// macro_rules! macro1 {
///     (a) => {
///         CaseA
///     };
///     (b) => {
///         CaseB
///     };
///     (CaseA) => {
///         MatchedCaseA
///     };
/// }
///
/// #[cps]
/// macro_rules! macro2 {
///     (a_b) =>
///     let $x:tt = macro1!(a) in
///     let $y:tt = macro1!(b) in
///     {
///         concat!($x, $y)
///     };
///
///     (sequential) =>
///     let $x:tt = macro1!(a) in
///     let $y:tt = macro2!($x) in
///     {
///         stringify!($y)
///     };
/// }
///
/// fn main() {
///     assert_eq!(macro2!(a_b), "CaseACaseB");
///     assert_eq!(macro2!(sequential), "MatchedCaseA");
/// }
/// ```
///
/// ## Macro indirection
///
/// The result of a previous let binding can be used as the name of a later let binding:
///
/// ```
/// # use cps::cps;
/// #[cps]
/// macro_rules! input_macro1 {
///     (next) => {
///         input_macro2
///     };
/// }
///
/// #[cps]
/// macro_rules! input_macro2 {
///     () => {
///         BaseCase2
///     };
/// }
///
/// #[cps]
/// macro_rules! macro1 {
///     ($cont1:ident) =>
///     let $cont2:ident = $cont1!(next) in
///     let $x:tt = $cont2!() in // Invoke the result of `$cont1!(next)`
///     {
///     stringify!($x)
///     };
/// }
///
/// fn main() {
///     assert_eq!(macro1!(input_macro1), "BaseCase2");
/// }
/// ```
///
/// [tlborm]: https://veykril.github.io/tlborm/decl-macros/patterns/callbacks.html
#[proc_macro_attribute]
pub fn cps(attr: TokenStream, item: TokenStream) -> TokenStream {
    let m = parse_macro_input!(item as ItemMacro);

    TokenStream::from(cps_macro::impl_cps(proc_macro2::TokenStream::from(attr), m))
}

macro_rules! impl_cps {
    (
        $(use $import:path;)*
        fn $name:ident($param_ident:ident : $param_ty:path $(,)?) -> $ret_ty:path {
            $( $impl_tt:tt )*
        }
    ) => {
        mod $name {
            $(use $import;)*

            pub struct Impl {}

            impl crate::cps_proc_macro::CPSProcMacro for Impl {
                type Input = $param_ty;
                type Output = $ret_ty;

                fn step($param_ident: $param_ty) -> $ret_ty {
                    $( $impl_tt )*
                }
            }
        }

        #[doc = "Performs the same task as the builtin macro of the same name, but this can also be used as a let binding in a CPS macro"]
        #[proc_macro]
        pub fn $name(item: TokenStream) -> TokenStream {
            let item = proc_macro2::TokenStream::from(item);
            let res = perform_macro::<$name::Impl>(item);
            TokenStream::from(res)
        }
    };
}

use crate::cps_proc_macro::perform_macro;

impl_cps!(
    fn stringify(tokens: proc_macro2::TokenStream) -> proc_macro2::Literal {
        proc_macro2::Literal::string(&tokens.to_string())
    }
);

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

impl_cps!(
    use syn::punctuated::Punctuated;
    use syn::Token;
    use crate::ConcatInput;

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
