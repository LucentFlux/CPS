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
    (1) => { BaseCase1 };
    (2) => { BaseCase2 };

    () =>
    // !!! NEW SYNTAX HERE !!!
    let $x:tt = foo!(1) in
    let $y:tt = foo!(2) in
    {
        concat!($x, " ", $y)
    };
}


fn main() {
    assert_eq!(foo!(), "BaseCase1 BaseCase2");
}
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

fn main() {
    println!("{}", dog_says!()); // Prints "dog!()", not "woof"
}
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

fn main() {
    println!("{}", dog_says!()); // Prints "woof" but is hard to read
}
```

While now having the correct behaviour, this is difficult to maintain as the flow of execution is confusing. Using CPS instead we get:

```rust
#[cps]
macro_rules! dog {
    () => {
        woof
    };
}

#[cps]
macro_rules! dog_says {
    () => 
    let $x::tt = dog!() in
    {
        stringify!($x)
    };
}

fn main() {
    println!("{}", dog_says!()); // Prints "woof"
}
```

## Reason 2 - Extendability

The `let` expressions in CPS macros must be built from other CPS macros, while the body mustn't. This allows us to add computation to be substituted in to macros developed by other people.
For example:

```rust
// Non-CPS macro from another crate
macro_rules! load_thing {
    ($path:expr) => {
        ...
    };
}

#[cps]
macro_rules! my_load_thing {
    ($path:expr) => 
    let $new_path::expr = cps::concat!("src/", $path) in
    {
        load_thing!($new_path)
    };
}
```

This crate comes with a collection of CPS macros that are copies of macros in the standard library, that can be used
to perform compile-time computation on token trees in a maintainable way.
