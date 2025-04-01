//! Foobar

use std::any::Any;
use std::marker::PhantomData;

use crate::BoxedArguments;
use crate::argument::ParseArguments;
use crate::argument::VerbArgument;
use crate::error::TestErrorCase;

/// A condition check for a given property
///
/// Conditions allow to check for anything you would find useful. For example, in a HTTP library,
/// you can check that your cache contains a valid entry from a previous request.
pub trait TestCondition<H>: std::fmt::Debug + Clone + 'static {
    /// The arguments for this condition
    type Arguments: ParseArguments<H>;

    /// Run the check now, may or may not actually be implemented
    ///
    /// This is only useful for non-transient properties. For example "is this connected". It is
    /// not a way to check for events.
    ///
    /// If the condition cannot properly support the concept of 'checking now' it is ok to simply
    /// return an error.
    fn check_now(&self, harness: &H, arguments: &Self::Arguments) -> miette::Result<bool>;

    /// Wait until a given condition evaluates to a meaningful value
    ///
    /// Some properties of a system are not meaningful other than 'that they happened'. This could
    /// be an event or some value changing.
    ///
    /// If the condition cannot properly support the concept of 'waiting until it has a value', it
    /// is ok to simply return an error.
    fn wait_until(&self, harness: &H, arguments: &Self::Arguments) -> miette::Result<bool>;
}

pub(crate) struct ErasedCondition<H> {
    condition: Box<dyn Any>,
    fn_parse_args:
        fn(&crate::TestDsl<H>, &kdl::KdlNode) -> Result<Box<dyn BoxedArguments<H>>, TestErrorCase>,
    fn_check_now: fn(&dyn Any, &H, &dyn Any) -> miette::Result<bool>,
    fn_wait_util: fn(&dyn Any, &H, &dyn Any) -> miette::Result<bool>,
    fn_clone: fn(&dyn Any) -> Box<dyn Any>,
}

impl<H> std::fmt::Debug for ErasedCondition<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErasedCondition")
            .field("condition", &self.condition)
            .field("fn_parse_args", &self.fn_parse_args)
            .field("fn_check_now", &self.fn_check_now)
            .field("fn_wait_util", &self.fn_wait_util)
            .field("fn_clone", &self.fn_clone)
            .finish()
    }
}

impl<H> Clone for ErasedCondition<H> {
    fn clone(&self) -> Self {
        Self {
            condition: (self.fn_clone)(&*self.condition),
            fn_parse_args: self.fn_parse_args,
            fn_check_now: self.fn_check_now,
            fn_wait_util: self.fn_wait_util,
            fn_clone: self.fn_clone,
        }
    }
}

impl<H> ErasedCondition<H> {
    pub(crate) fn erase<C>(condition: C) -> Self
    where
        C: TestCondition<H>,
    {
        ErasedCondition {
            condition: Box::new(condition),
            fn_parse_args: |test_dsl, node| {
                <C::Arguments as ParseArguments<H>>::parse(test_dsl, node).map(|a| {
                    let args = Box::new(a);
                    args as _
                })
            },
            fn_check_now: |this, harness, arguments| {
                let this: &C = this.downcast_ref().unwrap();
                let arguments: &C::Arguments = arguments.downcast_ref().unwrap();

                this.check_now(harness, arguments)
            },
            fn_wait_util: |this, harness, arguments| {
                let this: &C = this.downcast_ref().unwrap();
                let arguments: &C::Arguments = arguments.downcast_ref().unwrap();

                this.wait_until(harness, arguments)
            },
            fn_clone: |this| {
                let this: &C = this.downcast_ref().unwrap();

                Box::new(this.clone())
            },
        }
    }

    pub(crate) fn parse_args(
        &self,
        test_dsl: &crate::TestDsl<H>,
        node: &kdl::KdlNode,
    ) -> Result<Box<dyn BoxedArguments<H>>, TestErrorCase> {
        (self.fn_parse_args)(test_dsl, node)
    }

    pub(crate) fn check_now(&self, harness: &H, arguments: &dyn Any) -> miette::Result<bool> {
        (self.fn_check_now)(&*self.condition, harness, arguments)
    }
}

/// A [`Checker`] is the actual instance that executes when a condition evaluates.
///
/// It is mostly used with the [`Condition`] struct when given a closure/function.
///
/// It is implemented for closures of up to 16 arguments and their
pub trait Checker<H, T>: Clone + 'static {
    /// Execute the check with the given node
    fn check(&self, harness: &H, arguments: &T) -> miette::Result<bool>;
}

struct BoxedChecker<H, T> {
    checker: Box<dyn Any>,
    check_fn: fn(&dyn Any, harness: &H, node: &T) -> miette::Result<bool>,
    clone_fn: fn(&dyn Any) -> Box<dyn Any>,
}

impl<H, T> std::fmt::Debug for BoxedChecker<H, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedChecker")
            .field("checker", &self.checker)
            .field("check_fn", &self.check_fn)
            .field("clone_fn", &self.clone_fn)
            .finish()
    }
}

impl<H, T> BoxedChecker<H, T> {
    fn new<C>(checker: C) -> Self
    where
        C: Checker<H, T>,
    {
        BoxedChecker {
            checker: Box::new(checker),
            check_fn: |this, harness, node| {
                let this: &C = this.downcast_ref().unwrap();

                this.check(harness, node)
            },
            clone_fn: |this| {
                let this: &C = this.downcast_ref().unwrap();

                Box::new(this.clone())
            },
        }
    }

    fn check(&self, harness: &H, node: &T) -> miette::Result<bool> {
        (self.check_fn)(&*self.checker, harness, node)
    }
}

impl<H, T> Clone for BoxedChecker<H, T> {
    fn clone(&self) -> Self {
        BoxedChecker {
            checker: (self.clone_fn)(&*self.checker),
            check_fn: self.check_fn,
            clone_fn: self.clone_fn,
        }
    }
}

/// A condition that can be used in test-cases
///
/// Depending on how it is constructed, it may or may not be able to be used in direct or waiting
/// contexts
pub struct Condition<H, T> {
    now: Option<BoxedChecker<H, T>>,
    wait: Option<BoxedChecker<H, T>>,
    _pd: PhantomData<fn(H)>,
}

impl<H, T> std::fmt::Debug for Condition<H, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Condition")
            .field("now", &self.now)
            .field("wait", &self.wait)
            .field("_pd", &self._pd)
            .finish()
    }
}

impl<H, T> Condition<H, T> {
    /// Create a new [`Condition`] that can be called in direct contexts
    ///
    /// For example the `assert` verb allows you to verify multiple [`TestCondition`]s (of which [`Condition`] is one way to create one).
    pub fn new_now<C>(now: C) -> Self
    where
        C: Checker<H, T>,
    {
        Condition {
            now: Some(BoxedChecker::new(now)),
            wait: None,
            _pd: PhantomData,
        }
    }

    /// Create a new [`Condition`] that can be called in waiting contexts
    pub fn new_wait<C>(wait: C) -> Self
    where
        C: Checker<H, T>,
    {
        Condition {
            now: None,
            wait: Some(BoxedChecker::new(wait)),
            _pd: PhantomData,
        }
    }

    /// Create a new [`Condition`] that can be called in both direct and waiting contexts
    pub fn new_now_and_wait<C>(both: C) -> Self
    where
        C: Checker<H, T>,
    {
        Condition {
            now: Some(BoxedChecker::new(both.clone())),
            wait: Some(BoxedChecker::new(both)),
            _pd: PhantomData,
        }
    }

    /// Allow this condition to also be used in direct contexts
    pub fn with_now<C>(mut self, now: C) -> Self
    where
        C: Checker<H, T>,
    {
        self.now = Some(BoxedChecker::new(now));
        self
    }

    /// Allow this condition to also be used in waiting contexts
    pub fn with_wait<C>(mut self, wait: C) -> Self
    where
        C: Checker<H, T>,
    {
        self.wait = Some(BoxedChecker::new(wait));
        self
    }
}

impl<H, T> Clone for Condition<H, T> {
    fn clone(&self) -> Self {
        Condition {
            now: self.now.clone(),
            wait: self.wait.clone(),
            _pd: PhantomData,
        }
    }
}

impl<H, F> Checker<H, ((),)> for F
where
    F: Fn(&H) -> miette::Result<bool>,
    F: Clone + 'static,
{
    fn check(&self, harness: &H, _arguments: &((),)) -> miette::Result<bool> {
        self(harness)
    }
}

macro_rules! impl_callable {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[allow(non_snake_case, unused_mut)]
        impl<H, F, $($ty,)* $last> Checker<H, ($($ty,)* $last,)> for F
            where
                F: Fn(&H, $($ty,)* $last,) -> miette::Result<bool>,
                F: Clone + 'static,
                $( $ty: VerbArgument, )*
                $last: VerbArgument,
        {
            fn check(&self, harness: &H, node: &($($ty,)* $last,)) -> miette::Result<bool> {
                let ($($ty,)* $last,) = node.clone();
                self(harness, $($ty,)* $last,)
            }
        }
    };
}

all_the_tuples!(impl_callable);

impl<H, T> TestCondition<H> for Condition<H, T>
where
    H: 'static,
    T: ParseArguments<H>,
{
    type Arguments = T;
    fn check_now(&self, harness: &H, arguments: &T) -> miette::Result<bool> {
        let Some(check) = self.now.as_ref().map(|now| now.check(harness, arguments)) else {
            return Err(TestErrorCase::InvalidCondition {
                error: miette::miette!("Condition does not implement checking now"),
            }
            .into());
        };

        check
    }

    fn wait_until(&self, harness: &H, node: &T) -> miette::Result<bool> {
        let Some(check) = self.wait.as_ref().map(|wait| wait.check(harness, node)) else {
            return Err(TestErrorCase::InvalidCondition {
                error: miette::miette!("Condition does not implement checking now"),
            }
            .into());
        };

        check
    }
}
