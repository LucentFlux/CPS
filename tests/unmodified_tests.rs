use cps::cps;

#[cps]
macro_rules! macro1 {
    () => { "Empty Call" };

    (a) => { 1 };

    (12.5) => { 2 };

    (TestParens) => { ( 5 + 10) };

    (@ident $i:ident) => { $i };
}


#[test]
fn empty_preserved() {
    assert_eq!(macro1!(), "Empty Call");
}

#[test]
fn ident_preserved() {
    assert_eq!(macro1!(a), 1);
}

#[test]
fn literal_preserved() {
    assert_eq!(macro1!(12.5), 2);
}

#[test]
fn parens_returned() {
    assert_eq!(macro1!(TestParens) * 2, 30);
}

#[test]
fn ident_returned() {
    let my_var = -17;
    assert_eq!(macro1!(@ident my_var), my_var);
}