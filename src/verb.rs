//! Definition of verbs
//!
//! Verbs are the bread and butter of `test-dsl`. They define the behaviour that is then run
//! against your test harness.

use std::any::Any;
use std::marker::PhantomData;

use crate::BoxedArguments;
use crate::TestDsl;
use crate::argument::ParseArguments;
use crate::argument::VerbArgument;
use crate::error::TestErrorCase;

/// A verb is anything that 'does' things in a [`TestCase`](crate::test_case::TestCase)
pub trait Verb<H>: std::fmt::Debug + Clone + 'static {
    /// Arguments to this verb
    type Arguments: ParseArguments<H>;

    /// Run the verb, and do its thing
    fn run(&self, harness: &mut H, arguments: &Self::Arguments) -> miette::Result<()>;
}

pub(crate) struct ErasedVerb<H> {
    verb: Box<dyn Any>,
    fn_parse_args:
        fn(&crate::TestDsl<H>, &kdl::KdlNode) -> Result<Box<dyn BoxedArguments<H>>, TestErrorCase>,
    fn_run: fn(&dyn Any, &mut H, &dyn Any) -> miette::Result<()>,
    fn_clone: fn(&dyn Any) -> Box<dyn Any>,
}

impl<H> std::fmt::Debug for ErasedVerb<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErasedVerb")
            .field("verb", &self.verb)
            .field("fn_parse_args", &self.fn_parse_args)
            .field("fn_run", &self.fn_run)
            .field("fn_clone", &self.fn_clone)
            .finish()
    }
}

impl<H> Clone for ErasedVerb<H> {
    fn clone(&self) -> Self {
        Self {
            verb: (self.fn_clone)(&*self.verb),
            fn_parse_args: self.fn_parse_args,
            fn_run: self.fn_run,
            fn_clone: self.fn_clone,
        }
    }
}

impl<H> ErasedVerb<H> {
    pub(crate) fn erase<V>(verb: V) -> Self
    where
        V: Verb<H>,
    {
        ErasedVerb {
            verb: Box::new(verb),
            fn_parse_args: |test_dsl, node| {
                <V::Arguments as ParseArguments<H>>::parse(test_dsl, node).map(|a| {
                    let args = Box::new(a);
                    args as _
                })
            },
            fn_run: |this, harness, arguments| {
                let this: &V = this.downcast_ref().unwrap();
                let arguments: &V::Arguments = arguments.downcast_ref().unwrap();

                this.run(harness, arguments)
            },
            fn_clone: |this| {
                let this: &V = this.downcast_ref().unwrap();

                Box::new(this.clone())
            },
        }
    }

    pub(crate) fn parse_args(
        &self,
        test_dsl: &TestDsl<H>,
        node: &kdl::KdlNode,
    ) -> Result<Box<dyn BoxedArguments<H>>, TestErrorCase> {
        (self.fn_parse_args)(test_dsl, node)
    }

    pub(crate) fn run(&self, harness: &mut H, arguments: &dyn Any) -> miette::Result<()> {
        (self.fn_run)(&*self.verb, harness, arguments)
    }
}

/// A verb defined through a closure/function
///
/// See the [`CallableVerb`] trait for what can be used
pub struct FunctionVerb<H, T> {
    func: BoxedCallable<H, T>,
    _pd: PhantomData<fn(H, T)>,
}

impl<H, T> std::fmt::Debug for FunctionVerb<H, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionVerb")
            .field("func", &self.func)
            .field("_pd", &self._pd)
            .finish()
    }
}

impl<H, T> Clone for FunctionVerb<H, T> {
    fn clone(&self) -> Self {
        Self {
            func: self.func.clone(),
            _pd: self._pd,
        }
    }
}

impl<H, T> FunctionVerb<H, T> {
    /// Create a new verb using a closure/function
    pub fn new<F>(func: F) -> Self
    where
        F: CallableVerb<H, T>,
    {
        FunctionVerb {
            func: BoxedCallable::new(func),
            _pd: PhantomData,
        }
    }
}

struct BoxedCallable<H, T> {
    callable: Box<dyn Any>,
    call_fn: fn(&dyn Any, &mut H, &T) -> miette::Result<()>,
    clone_fn: fn(&dyn Any) -> Box<dyn Any>,
}

impl<H, T> std::fmt::Debug for BoxedCallable<H, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoxedCallable")
            .field("callable", &self.callable)
            .field("call_fn", &self.call_fn)
            .field("clone_fn", &self.clone_fn)
            .finish()
    }
}

impl<H, T> Clone for BoxedCallable<H, T> {
    fn clone(&self) -> Self {
        BoxedCallable {
            callable: (self.clone_fn)(&*self.callable),
            call_fn: self.call_fn,
            clone_fn: self.clone_fn,
        }
    }
}

impl<H, T> BoxedCallable<H, T> {
    fn new<F>(callable: F) -> Self
    where
        F: CallableVerb<H, T>,
    {
        BoxedCallable {
            callable: Box::new(callable),
            call_fn: |this, harness, node| {
                let this: &F = this.downcast_ref().unwrap();
                this.call(harness, node)
            },
            clone_fn: |this| {
                let this: &F = this.downcast_ref().unwrap();
                Box::new(this.clone())
            },
        }
    }

    fn call(&self, harness: &mut H, args: &T) -> miette::Result<()> {
        (self.call_fn)(&*self.callable, harness, args)
    }
}

/// Closure/functions that can be used as a Verb
///
/// This trait is implemented for closures with up to 16 arguments. They all have to be [`VerbArgument`]s.
pub trait CallableVerb<H, T>: Clone + 'static {
    /// Call the underlying closure
    fn call(&self, harness: &mut H, node: &T) -> miette::Result<()>;
}

impl<H, F, A> CallableVerb<H, ((), A)> for F
where
    F: Fn(&mut H, ((), A)) -> miette::Result<()>,
    F: Clone + 'static,
    A: ParseArguments<H>,
{
    fn call(&self, harness: &mut H, node: &((), A)) -> miette::Result<()> {
        self(harness, node.clone())
    }
}

impl<H, F> CallableVerb<H, ((),)> for F
where
    F: Fn(&mut H) -> miette::Result<()>,
    F: Clone + 'static,
{
    fn call(&self, harness: &mut H, _node: &((),)) -> miette::Result<()> {
        self(harness)
    }
}

macro_rules! impl_callable {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[allow(non_snake_case, unused_mut)]
        impl<H, F, $($ty,)* $last> CallableVerb<H, ($($ty,)* $last,)> for F
            where
                F: Fn(&mut H, $($ty,)* $last,) -> miette::Result<()>,
                F: Clone + 'static,
                $( $ty: VerbArgument, )*
                $last: VerbArgument,
        {
            fn call(&self, harness: &mut H, arguments: &($($ty,)* $last,)) -> miette::Result<()> {
                let ($($ty,)* $last,) = arguments.clone();
                self(harness, $($ty,)* $last,)
            }
        }
    };
}

all_the_tuples!(impl_callable);

impl<T, H: 'static> Verb<H> for FunctionVerb<H, T>
where
    T: ParseArguments<H>,
{
    type Arguments = T;
    fn run(&self, harness: &mut H, args: &T) -> miette::Result<()> {
        self.func.call(harness, args)
    }
}
