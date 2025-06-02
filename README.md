# CPS (AKA macro variables & let expressions)

[![crates.io](https://img.shields.io/crates/v/cps.svg)](https://crates.io/crates/cps)
[![docs.rs](https://img.shields.io/docsrs/cps)](https://docs.rs/cps/latest/cps/)
[![crates.io](https://img.shields.io/crates/l/cps.svg)](https://github.com/LucentFlux/cps/blob/main/LICENSE)

This crate allows for more traditional "inside-out" functions to be written in the rust macro syntax, allowing maintainable macros to be written in-line without needing to switch to a proc-macro.

TLDR:
```rust
use cps::cps;

#[cps] // Add this macro to unlock additional syntax
macro_rules! foo {
    (1) => { "BaseCase1" };
    (2) => { "BaseCase2" };

    () =>
    // !!! NEW SYNTAX HERE !!!
    let $x:tt = foo!(1) in
    let $y:tt = foo!(2) in
    {
        concat!($x, " ", $y)
    };
}

assert_eq!(foo!(), "BaseCase1 BaseCase2");
```

# Why?
## Reason 1 - Readability and Maintainability

Macro execution order is confusing. Because each macro is passed a token tree, macros execute outside-in. For example:

```rust
macro_rules! dog {
    () => {
        woof
    };
}

macro_rules! dog_says {
    () => 
    {
        stringify!(dog!())
    };
}

println!("{}", dog_says!()); // Prints "dog!()", not "woof"
```

Reading the above code as if macros are classical functions, you may expect this program to print `woof`. However unfortunately it prints `dog!()`, as if `println!` expands its macros while `stringify!` does not. This makes macros hard to maintain.

[The Little Book of Macros](https://veykril.github.io/tlborm/decl-macros/patterns/callbacks.html) describes *callbacks*, where a macro takes as an argument the next macro to execute. This leads to the following improved version of the above example:

```rust
macro_rules! dog {
    ($cont:ident) => {
        $cont!(woof)
    };
}

macro_rules! dog_says {
    () => 
    {
        dog!(stringify)
    };
}

println!("{}", dog_says!()); // Prints "woof" but is hard to read
```

While now having the correct behaviour, this is difficult to maintain as the flow of execution is confusing. Using CPS instead we get:

```rust
use cps::cps;

#[cps]
macro_rules! dog {
    () => {
        woof
    };
}

#[cps]
macro_rules! dog_says {
    () => 
    let $x:tt = dog!() in
    {
        stringify!($x)
    };
}

println!("{}", dog_says!()); // Prints "woof"
```

## Reason 2 - Extendability

The `let` expressions in CPS macros must be built from other CPS macros, while the body mustn't. This allows us to add computation to be substituted in to macros developed by other people.
For example:

```rust
use cps::cps;

// Non-CPS macro from another crate
macro_rules! load_thing {
    ($path:expr) => {
        ...
    };
}

#[cps]
macro_rules! my_load_thing {
    ($path:expr) => 
    let $new_path:expr = cps::concat!("src/", $path) in
    {
        load_thing!($new_path)
    };
}
```

This crate comes with a collection of CPS macros that are copies of macros in the standard library, that can be used
to perform compile-time computation on token trees in a maintainable way.

# Usage Notes

Macros-by-example are hard, difficult to maintain, and you should always consider writing a proc-macro instead.
This library aims to make the macros that you *do* write more maintainable. Please recurse responsibly.

CPS converts iteration into recursion. Therefore when using this library you may reach the recursion limit (128 at the time of writing). You can raise this using `#![recursion_limit = "1024"]` but your build times may suffer.

## Standard Library Macros

Any macro `let` expression must have a macro on the right-hand side that was marked as `#[cps]`. The following example will not work:

```rust
use cps::cps;

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

```rust
use cps::cps;

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

Note that the `include` and `include_str` macros resolve paths from the manifest file directory rather than the invocation location, due to the infamous [issue 54725](https://github.com/rust-lang/rust/issues/54725).

## Portability

CPS macros are portable, meaning they can be invoked inside of crates that do not use the `cps` crate. However their dependent macros are not inlined, so all dependent macro crates must be in scope. This includes `cps` if any of the standard library macros are used. Therefore the following code is recommended:

```rust
#[doc(hidden)]
pub use cps::cps;

#[cps]
macro_rules! foo {
    () => { BaseCase };

    (bar) =>
    let $x:tt = foo!() in
    let $y:tt = $crate::cps::stringify!($x) in // Refer to a version of `cps` that we bring along with us.
    {
        $y
    };
}
```

Not following this pattern usually results in an error like ``` error[E0433]: failed to resolve: use of unresolved module or unlinked crate `cps` ```.