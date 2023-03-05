# How this all works

To implement proper execution order we need a stack. We can implement this stack within a rust macro with custom match patterns that each progress execution by one step. This implements iteration through recursion, and also recursion through recursion, using Continuation Passing Style (the origin of this crate's name).

A simple example:

```rust
#[cps]
macro_rules! example {
    () => {
        Foo
    };
}
```

should become:

```rust
macro_rules! example {
  // The base case of recursion:
  // - empty call stack (so execution is done)
  // - one item on the data stack (since there are no let bindings)
  // - no parameters in the top of the data stack (because the macro takes no arguments)
  // at which point we evaluate the body
  (@_cps |:|  |:| {} | ) => {
    Foo
  };

  // Execution intermediate step:
  // - the call stack has one or more items left to evaluate
  // - there is an empty parameter set on the top of our data stack
  // so we evaluate the body on to the call stack and continue execution with the next call on the stack
  (@_cps |:| ($_cps_next_head:tt) $(| ($_cps_next_tail:tt))* |:| {} | $($_cps_stack:tt)*) => {
    $_cps_next_head!{
      @_cps |:| $(($_cps_next_tail))|* |:| { Foo } $($_cps_stack)*
    }
  };

  // Entry case - create a stack and start execution
  () => {
    example!{@_cps |:|  |:| {} | }
  }
}
```

The actual macros also have some more cases at the end to catch errors and report them in a nice way, as 'runtime' compile errors should be.