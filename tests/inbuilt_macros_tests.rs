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
}

#[test]
fn stringify_call() {
    assert_eq!(macro1!(@run stringify), "Got: BaseCase");
}

#[test]
fn concat_call() {
    assert_eq!(macro1!(@run concat), "Got: 3BaseCasetrue");
}
