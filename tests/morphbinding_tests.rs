use cps::cps;

#[cps]
macro_rules! input_macro1 {
    () => {
        BaseCase1
    };

    (next) => {
        input_macro2
    };
}

#[cps]
macro_rules! input_macro2 {
    () => {
        BaseCase2
    };
}

#[cps]
macro_rules! macro1 {
    ($cont:ident) =>
    let $x:tt = $cont!() in
    {
        stringify!($x)
    };

    (do two $cont1:ident) =>
    let $cont2:ident = $cont1!(next) in
    let $x:tt = $cont2!() in
    {
        stringify!($x)
    };
}

#[test]
fn stringify_order_single_call1() {
    assert_eq!(macro1!(input_macro1), "BaseCase1");
}

#[test]
fn stringify_order_single_call2() {
    assert_eq!(macro1!(input_macro2), "BaseCase2");
}

#[test]
fn stringify_order_progress_through() {
    assert_eq!(macro1!(do two input_macro1), "BaseCase2");
}
