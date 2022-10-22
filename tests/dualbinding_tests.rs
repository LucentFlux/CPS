use cps::cps;

#[cps]
macro_rules! macro1 {
    (a) => { CaseA };
    (b) => { CaseB };

    (CaseA) => { MatchedCaseA };

    (stringify_two) =>
    let $x:tt = macro1!(a) in
    let $y:tt = macro1!(b) in
    {
        stringify!($x).to_owned() + stringify!($y)
    };

    (stringify_sequential) =>
    let $x:tt = macro1!(a) in
    let $y:tt = macro1!($x) in
    {
        stringify!($y)
    };
}


#[test]
fn stringify_order_single_call() {
    assert_eq!(macro1!(stringify_two), "CaseACaseB");
}

#[test]
fn stringify_order_sequential_call() {
    assert_eq!(macro1!(stringify_sequential), "MatchedCaseA");
}
