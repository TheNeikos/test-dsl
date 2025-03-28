use std::marker::PhantomData;

use crate::error::TestErrorCase;

pub trait TestCondition<H>: 'static {
    fn check_now(&self, harness: &H, node: &kdl::KdlNode) -> Result<(), TestErrorCase>;

    fn wait_until_true(&self, harness: &H, node: &kdl::KdlNode) -> Result<(), TestErrorCase>;

    fn clone_box(&self) -> Box<dyn TestCondition<H>>;
}

trait Checker<H>: Clone {
    fn check(&self, harness: &H) -> Option<bool>;
}

impl<H> Checker<H> for () {
    fn check(&self, _harness: &H) -> Option<bool> {
        None
    }
}

impl<T, H> Checker<H> for T
where
    T: Clone,
    T: Fn(&H) -> bool,
{
    fn check(&self, harness: &H) -> Option<bool> {
        Some((self)(harness))
    }
}

pub struct Condition<H, N, W> {
    now: N,
    wait: W,
    _pd: PhantomData<fn(N, W, H)>,
}

impl<H, N> Condition<H, N, ()>
where
    N: Fn(&H) -> bool,
{
    pub fn new_now(now: N) -> Self {
        Condition {
            now,
            wait: (),
            _pd: PhantomData,
        }
    }
}

impl<H, N: Clone, W: Clone> Clone for Condition<H, N, W> {
    fn clone(&self) -> Self {
        Condition {
            now: self.now.clone(),
            wait: self.wait.clone(),
            _pd: PhantomData,
        }
    }
}

impl<H, N, W> TestCondition<H> for Condition<H, N, W>
where
    H: 'static,
    N: 'static,
    W: 'static,
    N: Checker<H>,
    W: Checker<H>,
{
    fn check_now(&self, harness: &H, node: &kdl::KdlNode) -> Result<(), TestErrorCase> {
        let Some(check) = self.now.check(harness) else {
            return Err(TestErrorCase::Error {
                error: miette::miette!("Condition does not implement checking now"),
                label: node.span(),
            });
        };

        check
            .then_some(())
            .ok_or_else(|| crate::error::TestErrorCase::Error {
                error: miette::miette!("Check did not succeed"),
                label: node.span(),
            })
    }

    fn wait_until_true(&self, harness: &H, node: &kdl::KdlNode) -> Result<(), TestErrorCase> {
        let Some(check) = self.wait.check(harness) else {
            return Err(TestErrorCase::Error {
                error: miette::miette!("Condition does not implement waiting"),
                label: node.span(),
            });
        };

        check
            .then_some(())
            .ok_or_else(|| crate::error::TestErrorCase::Error {
                error: miette::miette!("Check did not succeed"),
                label: node.span(),
            })
    }

    fn clone_box(&self) -> Box<dyn TestCondition<H>> {
        Box::new(self.clone())
    }
}
