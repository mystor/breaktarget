//! The `breaktarget` module defines one type: The `BreakTarget` type. A
//! value of this type may be obtained by invoking `BreakTarget::deploy`.
//!
//! `BreakTarget::deploy` takes a lambda, which is passed a reference to a
//! `BreakTarget`. This object defines a single method, `break_with`, which
//! takes a value of type `T` and returns control flow to the site of the
//! `BreakTarget::deploy` call, producing the value as the result. If the lambda
//! exits normally, it must also produce a value of type `T`, which is produced
//! as the result of the `BreakTarget::deploy` call.
//!
//! # Important Notes
//!
//! The nonlocal breaking is implemented using a panic to unwind the stack. Any
//! open Mutexes which are closed by this unwinding will be poisoned, among with
//! other unwinding specific effects.
//!
//! _If `panic = abort` is enabled, calls to `break_with` will abort the program_
//!
//! # Examples
//!
//! ```
//! use breaktarget::BreakTarget;
//!
//! let result = BreakTarget::deploy(|target| {
//!     // ... Some logic here
//!     target.break_with(10i32);
//!     // ... Some logic here
//! });
//! assert_eq!(result, 10i32);
//! ```
//!
//! ```
//! use breaktarget::BreakTarget;
//!
//! fn some_function(target: &BreakTarget<i32>, cond: bool) {
//!    if cond {
//!        target.break_with(10);
//!    }
//! }
//!
//! let result1 = BreakTarget::deploy(|target| {
//!     some_function(target, false);
//!     20
//! });
//! assert_eq!(result1, 20);
//!
//! let result2 = BreakTarget::deploy(|target| {
//!     some_function(target, true);
//!     20
//! });
//! assert_eq!(result2, 10);
//! ```

use std::panic;
use std::cell::RefCell;

/// A BreakRequest is a dummy zero-sized-type. It's heap address is used to
/// identify which BreakTarget we are breaking towards.
struct BreakRequest;

/// This object represents the target stack frame which we will unwind toward
/// when the break_with method is invoked. The value which we are breaking with
/// will be stored within the BreakTarget to be returned when control flow
/// resumes.
#[derive(Debug)]
pub struct BreakTarget<T>(RefCell<Option<T>>);

impl<T> BreakTarget<T> {
    /// Deploy a break target. The target will be passed by reference to the
    /// argument closure. The BreakTarget object provides a single `break_with`
    /// method, which can be invoked to halt execution and return control to the
    /// deployment site. If the `break_with` function was not invoked, the
    /// return value of the closure will instead be produced.
    pub fn deploy<F>(func: F) -> T where F: FnOnce(&BreakTarget<T>) -> T {
        // A place for storing the information if the function aborts during its
        // execution. The address of this local is also used as a marker value
        // for the panic value when break_with is called, allowing us to resume
        // without parforming somewhat expensive downcasts.
        let target = BreakTarget(RefCell::new(None));

        // Run the logic, catching any panics triggered
        match panic::catch_unwind(panic::AssertUnwindSafe(|| func(&target))) {
            Ok(v) => v,
            Err(panic_val) => {
                if let Some(panic_ptr) = panic_val.downcast_ref::<BreakRequest>() {
                    // Check if the panic we got back has a data pointer which
                    // refers to our break target. If it does, it was triggered
                    // by our break_with function.
                    if panic_ptr as *const _ as *const Self == &target as *const _ {
                        return target.0.into_inner().unwrap();
                    }
                }

                panic::resume_unwind(panic_val);
            }
        }
    }

    /// Aborts the current function, returning control to the BreakTarget's
    /// deploy point. The argument to this method will be the return value of
    /// the deploy method.
    pub fn break_with(&self, data: T) -> ! {
        // Record the information in the continuation object
        *self.0.borrow_mut() = Some(data);

        // Create an unwind sentinel object. Use our address as the address for
        // the zero sized type BreakRequest such that we can communicate that
        // we are the Continuation which is being triggered, while not breaking
        // anything, as BreakRequest won't actually allocate any memory on the
        // heap, and thus the box destructor will be a no-op.
        let unwind_box: Box<BreakRequest> = unsafe {
            Box::from_raw(self as *const Self as *mut Self as *mut BreakRequest)
        };

        // Use the resume_unwind function to unwind rather than panic! such that
        // the object isn't double-boxed,
        panic::resume_unwind(unwind_box);
    }
}

#[cfg(test)]
mod tests {
    use super::BreakTarget;
    use std::panic;

    #[test]
    fn basic_use() {
        let mut before = false;
        let res = BreakTarget::deploy(|t| {
            before = true;
            t.break_with(1);
        });
        assert_eq!(res, 1);
        assert!(before);
    }

    #[test]
    fn unwind_to_outer() {
        let res = BreakTarget::deploy(|t| {
            BreakTarget::deploy(|_| t.break_with(1));
            unreachable!();
        });
        assert_eq!(res, 1);
    }

    #[test]
    fn propagate_panic() {
        // Ensure that panics produced within a BreakTarget::deploy are propagated to caller
        if let Err(e) = panic::catch_unwind(|| BreakTarget::deploy(|_| panic!(1u32))) {
            assert_eq!(e.downcast_ref::<u32>(), Some(&1u32));
        } else {
            assert!(false, "should panic");
        }
    }
}
