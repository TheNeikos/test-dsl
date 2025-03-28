use std::any::Any;
use std::marker::PhantomData;

use crate::arguments::VerbArgument;
use crate::error::TestErrorCase;

pub trait TestCondition<H>: 'static {
    fn check_now(&self, harness: &H, node: &kdl::KdlNode) -> Result<bool, TestErrorCase>;

    fn wait_until(&self, harness: &H, node: &kdl::KdlNode) -> Result<bool, TestErrorCase>;

    fn clone_box(&self) -> Box<dyn TestCondition<H>>;
}

pub trait Checker<H, T>: Clone + 'static {
    fn check(&self, harness: &H, node: &kdl::KdlNode) -> Result<bool, TestErrorCase>;
}

struct BoxedChecker<H> {
    checker: Box<dyn Any>,
    check_fn: fn(&dyn Any, harness: &H, node: &kdl::KdlNode) -> Result<bool, TestErrorCase>,
    clone_fn: fn(&dyn Any) -> Box<dyn Any>,
}

impl<H> BoxedChecker<H> {
    fn new<C, T>(checker: C) -> Self
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

    fn check(&self, harness: &H, node: &kdl::KdlNode) -> Result<bool, TestErrorCase> {
        (self.check_fn)(&*self.checker, harness, node)
    }
}

impl<H> Clone for BoxedChecker<H> {
    fn clone(&self) -> Self {
        BoxedChecker {
            checker: (self.clone_fn)(&*self.checker),
            check_fn: self.check_fn,
            clone_fn: self.clone_fn,
        }
    }
}

pub struct Condition<H> {
    now: Option<BoxedChecker<H>>,
    wait: Option<BoxedChecker<H>>,
    _pd: PhantomData<fn(H)>,
}

impl<H> Condition<H> {
    pub fn new_now<C, T>(now: C) -> Self
    where
        C: Checker<H, T>,
    {
        Condition {
            now: Some(BoxedChecker::new(now)),
            wait: None,
            _pd: PhantomData,
        }
    }
}

impl<H> Clone for Condition<H> {
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
    fn check(&self, harness: &H, node: &kdl::KdlNode) -> Result<bool, TestErrorCase> {
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
        impl<H, F, $($ty,)* $last> Checker<H, ($($ty,)* $last,)> for F
            where
                F: Fn(&H, $($ty,)* $last,) -> miette::Result<bool>,
                F: Clone + 'static,
                $( $ty: VerbArgument, )*
                $last: VerbArgument,
        {
            fn check(&self, harness: &H, node: &kdl::KdlNode) -> Result<bool, TestErrorCase> {
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
                        missing: format!("This condition takes {} arguments, you're missing the {}th argument.", total_count, running_count),
                    })?;

                    let $ty = <$ty as VerbArgument>::from_value(arg).ok_or_else(|| {
                        TestErrorCase::WrongArgumentType {
                            parent: node.name().span(),
                            argument: arg.span(),
                            expected: format!("This condition takes a '{}' as its argument here.", <$ty as VerbArgument>::TYPE_NAME),
                        }
                    })?;
                    running_count += 1;
                )*

                let _ = running_count;

                let arg = args.next().ok_or_else(|| TestErrorCase::MissingArgument {
                    parent: node.span(),
                    missing: format!("This condition takes {tc} arguments, you're missing the {tc}th argument.", tc = total_count),
                })?;
                let $last = <$last as VerbArgument>::from_value(arg).ok_or_else(|| {
                    TestErrorCase::WrongArgumentType {
                        parent: node.name().span(),
                        argument: arg.span(),
                        expected: format!("This condition takes a '{}' as its argument here.", <$last as VerbArgument>::TYPE_NAME),
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

impl<H> TestCondition<H> for Condition<H>
where
    H: 'static,
{
    fn check_now(&self, harness: &H, node: &kdl::KdlNode) -> Result<bool, TestErrorCase> {
        let Some(check) = self.now.as_ref().map(|now| now.check(harness, node)) else {
            return Err(TestErrorCase::Error {
                error: miette::miette!("Condition does not implement checking now"),
                label: node.span(),
            });
        };

        check
    }

    fn wait_until(&self, harness: &H, node: &kdl::KdlNode) -> Result<bool, TestErrorCase> {
        let Some(check) = self.wait.as_ref().map(|wait| wait.check(harness, node)) else {
            return Err(TestErrorCase::Error {
                error: miette::miette!("Condition does not implement waiting"),
                label: node.span(),
            });
        };

        check
    }

    fn clone_box(&self) -> Box<dyn TestCondition<H>> {
        Box::new(self.clone())
    }
}
