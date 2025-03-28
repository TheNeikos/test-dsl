use std::any::Any;
use std::marker::PhantomData;

use crate::arguments::VerbArgument;
use crate::error::TestRunResultError;

pub trait TestVerb<H>: 'static {
    fn run(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestRunResultError>;
    fn clone_box(&self) -> Box<dyn TestVerb<H>>;
}

impl<H: 'static> Clone for Box<dyn TestVerb<H>> {
    fn clone(&self) -> Self {
        let this: &dyn TestVerb<H> = &**self;
        this.clone_box()
    }
}

pub struct FunctionVerb<H> {
    func: BoxedCallable<H>,
    _pd: PhantomData<fn(H)>,
}

impl<H> Clone for FunctionVerb<H> {
    fn clone(&self) -> Self {
        Self {
            func: self.func.clone(),
            _pd: self._pd,
        }
    }
}

impl<H> FunctionVerb<H> {
    pub fn new<F, T>(func: F) -> Self
    where
        F: CallableVerb<H, T>,
    {
        FunctionVerb {
            func: BoxedCallable::new(func),
            _pd: PhantomData,
        }
    }
}

struct BoxedCallable<H> {
    callable: Box<dyn Any>,
    call_fn: fn(&dyn Any, &mut H, &kdl::KdlNode) -> Result<(), TestRunResultError>,
    clone_fn: fn(&dyn Any) -> Box<dyn Any>,
}

impl<H> Clone for BoxedCallable<H> {
    fn clone(&self) -> Self {
        BoxedCallable {
            callable: (self.clone_fn)(&*self.callable),
            call_fn: self.call_fn,
            clone_fn: self.clone_fn,
        }
    }
}

impl<H> BoxedCallable<H> {
    fn new<F, T>(callable: F) -> Self
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

    fn call(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestRunResultError> {
        (self.call_fn)(&*self.callable, harness, node)
    }
}

pub trait CallableVerb<H, T>: Clone + 'static {
    fn call(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestRunResultError>;
}

impl<H, F> CallableVerb<H, ((),)> for F
where
    F: Fn(&mut H) -> miette::Result<()>,
    F: Clone + 'static,
{
    fn call(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestRunResultError> {
        self(harness).map_err(|error| TestRunResultError::Error {
            error,
            label: node.span(),
        })
    }
}

#[rustfmt::skip]
macro_rules! all_the_tuples {
    ($name:ident) => {
        $name!([], T1);
        $name!([T1], T2);
        $name!([T1, T2], T3);
        $name!([T1, T2, T3], T4);
        $name!([T1, T2, T3, T4], T5);
        $name!([T1, T2, T3, T4, T5], T6);
        $name!([T1, T2, T3, T4, T5, T6], T7);
        $name!([T1, T2, T3, T4, T5, T6, T7], T8);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8], T9);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9], T10);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10], T11);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11], T12);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12], T13);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13], T14);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14], T15);
        $name!([T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15], T16);
    };
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
            fn call(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestRunResultError> {
                let mut args = node.iter();

                $(
                    let $ty = <$ty as VerbArgument>::from_value(args.next().unwrap()).unwrap();
                )*

                let $last = <$last as VerbArgument>::from_value(args.next().unwrap()).unwrap();

                self(harness, $($ty,)* $last,).map_err(|error| TestRunResultError::Error {
                    error,
                    label: node.span()
                })
            }
        }
    };
}

all_the_tuples!(impl_callable);

impl<H: 'static> TestVerb<H> for FunctionVerb<H> {
    fn run(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestRunResultError> {
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.func.call(harness, node)
        }));

        match res {
            Ok(res) => res,
            Err(error) => {
                let mut message = "Something went wrong".to_string();

                let payload = error;

                if let Some(msg) = payload.downcast_ref::<&str>() {
                    message = msg.to_string();
                }

                if let Some(msg) = payload.downcast_ref::<String>() {
                    message.clone_from(msg);
                }

                Err(TestRunResultError::Panic {
                    error: miette::Error::msg(message),
                    label: node.span(),
                })
            }
        }
    }

    fn clone_box(&self) -> Box<dyn TestVerb<H>> {
        Box::new(self.clone())
    }
}
