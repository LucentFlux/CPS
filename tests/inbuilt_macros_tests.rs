use cps::cps;

#[cps]
macro_rules! macro1 {
    () => { BaseCase };

    ($expr:expr) => { concat!("Got: ", $expr) };

    (@run stringify) =>
    let $x:tt = macro1!() in
    let $y:expr = cps::stringify!($x) in
    let $($z:tt)* = macro1!($y) in
    {
        $($z)*
    };

    (@run concat) =>
    let $x:tt = macro1!() in
    let $a:expr = cps::stringify!($x) in
    let $y:expr = cps::concat!(3, $a, true) in
    let $($z:tt)* = macro1!($y) in
    {
        $($z)*
    };

    (@run include) =>
    let $($y:tt)* = cps::include_str!("tests/test_file.txt") in
    let $z:tt = cps::stringify!($($y)*) in
    {
        $z
    };
}

#[test]
fn stringify_call() {
    assert_eq!(macro1!(@run stringify), "Got: BaseCase");
}

#[test]
fn concat_call() {
    assert_eq!(macro1!(@run concat), "Got: 3BaseCasetrue");
}

#[test]
fn include_call() {
    assert_eq!(
        macro1!(@run include),
        "this is a test file used for include macros testing!"
    );
}
