# breaktarget

[![Build Status](https://travis-ci.org/mystor/breaktarget.svg?branch=master)](https://travis-ci.org/mystor/breaktarget)

The `breaktarget` module defines one type: The `BreakTarget` type. A
value of this type may be obtained by invoking `BreakTarget::deploy`.

`BreakTarget::deploy` takes a lambda, which is passed a reference to a
`BreakTarget`. This object defines a single method, `break_with`, which
takes a value of type `T` and returns control flow to the site of the
`BreakTarget::deploy` call, producing the value as the result. If the lambda
exits normally, it must also produce a value of type `T`, which is produced
as the result of the `BreakTarget::deploy` call.

# Important Notes

The nonlocal breaking is implemented using a panic to unwind the stack. Any
open Mutexes which are closed by this unwinding will be poisoned, among with
other unwinding specific effects.

## If `panic = "abort"` is enabled, calls to `break_with` will abort the program

# Examples

```rust
use breaktarget::Breaktarget;

let result = BreakTarget::deploy(|target| {
    // ... Some logic here
    target.break_with(10i32);
    // ... Some logic here
});
assert_eq!(result, 10i32);
```

```rust
use breaktarget::BreakTarget;

fn some_function(target: &BreakTarget<i32>, cond: bool) {
   if cond {
       target.break_with(10);
   }
}

let result1 = BreakTarget::deploy(|target| {
    some_function(target, false);
    20
});
assert_eq!(result1, 20);

let result2 = BreakTarget::deploy(|target| {
    some_function(target, true);
    20
});
assert_eq!(result2, 10);
```
