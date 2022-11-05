use cps::cps;

#[cps]
macro_rules! macro1 {
    ($callback:ident) =>
    let $($v1:tt)* = $callback!(call1) in
    let $($v2:tt)* = $callback!(call2) in
    {
        concat!($($v1)*, $($v2)*)
    }
}

#[cps]
macro_rules! macro2 {
    (call1) => { "A" };
    (call2) => { "B" };

    () =>
    let $($v1:tt)* = macro1!(macro2) in
    let $($v2:tt)* = macro1!(macro2) in
    {
        concat!($($v1)*, $($v2)*)
    }
}

#[test]
fn stringify_order_single_call() {
    assert_eq!(macro2!(), "ABAB");
}
