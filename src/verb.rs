use std::any::Any;
use std::marker::PhantomData;

use crate::arguments::VerbArgument;
use crate::error::TestErrorCase;

pub trait TestVerb<H>: 'static {
    fn run(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestErrorCase>;
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
    call_fn: fn(&dyn Any, &mut H, &kdl::KdlNode) -> Result<(), TestErrorCase>,
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

    fn call(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestErrorCase> {
        (self.call_fn)(&*self.callable, harness, node)
    }
}

pub trait CallableVerb<H, T>: Clone + 'static {
    fn call(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestErrorCase>;
}

impl<H, F> CallableVerb<H, ((),)> for F
where
    F: Fn(&mut H) -> miette::Result<()>,
    F: Clone + 'static,
{
    fn call(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestErrorCase> {
        self(harness).map_err(|error| TestErrorCase::Error {
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
            fn call(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestErrorCase> {
                let mut args = node.iter();

                let total_count = 1
                    $(
                        + {
                            const _: () = {
                                #[allow(unused)]
                                let $ty = ();
                            };
                            1
                        }

                    )*;

                let mut running_count = 1;

                $(
                    let arg = args.next().ok_or_else(|| TestErrorCase::MissingArgument {
                        parent: node.span(),
                        missing: format!("This function takes {} arguments, you're missing the {}th argument.", total_count, running_count),
                    })?;

                    let $ty = <$ty as VerbArgument>::from_value(arg).ok_or_else(|| {
                        TestErrorCase::WrongArgumentType {
                            parent: node.name().span(),
                            argument: arg.span(),
                            expected: format!("This function takes a '{}' as its argument here.", <$ty as VerbArgument>::TYPE_NAME),
                        }
                    })?;
                    running_count += 1;
                )*

                let _ = running_count;

                let arg = args.next().ok_or_else(|| TestErrorCase::MissingArgument {
                    parent: node.span(),
                    missing: format!("This function takes {tc} arguments, you're missing the {tc}th argument.", tc = total_count),
                })?;
                let $last = <$last as VerbArgument>::from_value(arg).ok_or_else(|| {
                    TestErrorCase::WrongArgumentType {
                        parent: node.name().span(),
                        argument: arg.span(),
                        expected: format!("This function takes a '{}' as its argument here.", <$last as VerbArgument>::TYPE_NAME),
                    }
                })?;

                self(harness, $($ty,)* $last,).map_err(|error| TestErrorCase::Error {
                    error,
                    label: node.span()
                })
            }
        }
    };
}

all_the_tuples!(impl_callable);

impl<H: 'static> TestVerb<H> for FunctionVerb<H> {
    fn run(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), TestErrorCase> {
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

                Err(TestErrorCase::Panic {
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
