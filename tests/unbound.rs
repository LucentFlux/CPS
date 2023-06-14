// We should allow patterns that don't actually bind a repitition

#[cps::cps]
macro_rules! macro1 {
    (a) => {
        "Case1"
    };
    (a,) => {
        "Case2"
    };
    (a $(,)*) => {
        "Case3"
    };
    (b) =>
    {
        "Case4"
    };
    (b,) =>
    {
        "Case5"
    };
    (b $(,)*) =>
    let $v:tt = macro1!(a) in
    {
        concat!($v, "Case6")
    };
}

#[test]
fn case_1() {
    assert_eq!(macro1!(a), "Case1");
}

#[test]
fn case_2() {
    assert_eq!(macro1!(a,), "Case2");
}

#[test]
fn case_3() {
    assert_eq!(macro1!(a,,), "Case3");
}

#[test]
fn case_4() {
    assert_eq!(macro1!(b,,), "Case1Case6");
}
