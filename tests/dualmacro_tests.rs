use cps::cps;

#[cps]
macro_rules! macro1 {
    (a) => {
        Case1A
    };
    (b) => {
        Case1B
    };

    (Case1A) => {
        Matched1Case1A
    };
    (Case2A) => {
        Matched1Case2A
    };
}

#[cps]
macro_rules! macro2 {
    (a) => { Case2A };
    (b) => { Case2B };

    (Case1A) => { Matched2Case1A };
    (Case2A) => { Matched2Case2A };

    (stringify_1_2) =>
    let $x:tt = macro1!(a) in
    let $y:tt = macro2!(b) in
    {
        stringify!($x).to_owned() + stringify!($y)
    };

    (stringify_2_1) =>
    let $x:tt = macro2!(a) in
    let $y:tt = macro1!(b) in
    {
        stringify!($x).to_owned() + stringify!($y)
    };

    (stringify_sequential_1_2) =>
    let $x:tt = macro1!(a) in
    let $y:tt = macro2!($x) in
    {
        stringify!($y)
    };

    (stringify_sequential_2_1) =>
    let $x:tt = macro2!(a) in
    let $y:tt = macro1!($x) in
    {
        stringify!($y)
    };
}

#[test]
fn stringify_1_2_call() {
    assert_eq!(macro2!(stringify_1_2), "Case1ACase2B");
}

#[test]
fn stringify_2_1_call() {
    assert_eq!(macro2!(stringify_2_1), "Case2ACase1B");
}

#[test]
fn stringify_sequential_1_2_call() {
    assert_eq!(macro2!(stringify_sequential_1_2), "Matched2Case1A");
}

#[test]
fn stringify_sequential_2_1_call() {
    assert_eq!(macro2!(stringify_sequential_2_1), "Matched1Case2A");
}
