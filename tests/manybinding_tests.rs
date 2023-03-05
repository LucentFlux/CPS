use cps::cps;

#[cps]
macro_rules! macro1 {
    (a) => {
        CaseA
    };
    (b) => {
        CaseB
    };
    (CaseA) => {
        MatchedCaseA
    };
}

#[cps]
macro_rules! macro2 {
    (a_b) =>
    let $x:tt = macro1!(a) in
    let $x2:tt = cps::stringify!($x) in
    let $y:tt = macro1!(b) in
    let $y2:tt = cps::stringify!($y) in
    {
        concat!($x2, $y2)
    };
}

#[test]
fn stringify_in_order() {
    assert_eq!(macro2!(a_b), "CaseACaseB");
}
