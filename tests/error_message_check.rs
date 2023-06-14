// This is a manual test, since it results in a compiler error.

#[cps::cps]
macro_rules! twice_error_message {
    ($($branch_one:ident)|*) => {};
    ($($branch_two:ident),*) => {};
}

//twice_error_message!(A B);
