#![deny(missing_docs)]
#![doc=::std::include_str!("../README.md")]

mod cps_macro;
mod cps_proc_macro;
mod parse_cps_input;
mod parse_macro_decl;
mod std_macros;

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemMacro};

/// Manipulates a macro_rules! definition to add extended syntax to help in creating readable macros.
///
/// # Usage
///
/// CPS macros are a strict superset of Rust macro-rules. This means that any (\*) regular rust macro can
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
///     let $x2:tt = cps::stringify!($x) in
///     let $y:tt = macro1!(b) in
///     let $y2:tt = cps::stringify!($y) in
///     {
///         concat!($x2, $y2)
///     };
///
///     (sequential) =>
///     let $x:tt = macro1!(a) in
///     let $y:tt = macro1!($x) in
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

macro_rules! export_std_cps {
    ($name:ident) => {

        #[doc = "Performs the same task as the builtin macro of the same name, but this version can also be used as a let binding in a CPS macro"]
        #[proc_macro]
        pub fn $name(item: TokenStream) -> TokenStream {
            crate::std_macros::$name::$name(item)
        }
    };
}

export_std_cps!(concat);
export_std_cps!(stringify);
export_std_cps!(include_str);
