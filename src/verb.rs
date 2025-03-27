use std::marker::PhantomData;

use crate::arguments::VerbArgument;
use crate::error::TestParseError;
use crate::error::TestParseErrorCase;
use crate::error::TestRunResult;
use crate::error::TestRunResultError;

pub trait TestVerb<H>: 'static {
    fn run(
        &self,
        harness: &mut H,
        node: &kdl::KdlNode,
        arguments: &[kdl::KdlEntry],
    ) -> Result<TestRunResult, TestParseError>;
    fn clone_box(&self) -> Box<dyn TestVerb<H>>;
}

impl<H: 'static> Clone for Box<dyn TestVerb<H>> {
    fn clone(&self) -> Self {
        let this: &dyn TestVerb<H> = &**self;
        this.clone_box()
    }
}

pub struct FunctionVerb<H, F, Args> {
    pub(crate) func: F,
    pub(crate) _pd: PhantomData<fn(H, Args)>,
}

impl<H, F: Clone, Args> Clone for FunctionVerb<H, F, Args> {
    fn clone(&self) -> Self {
        Self {
            func: self.func.clone(),
            _pd: self._pd,
        }
    }
}

impl<H, F> From<F> for FunctionVerb<H, F, ()>
where
    F: Fn(&mut H),
{
    fn from(value: F) -> Self {
        FunctionVerb {
            func: value,
            _pd: PhantomData,
        }
    }
}

impl<H, F> From<F> for FunctionVerb<H, F, (usize,)>
where
    F: Fn(&mut H, usize),
{
    fn from(value: F) -> Self {
        FunctionVerb {
            func: value,
            _pd: PhantomData,
        }
    }
}

impl<F, H: 'static> TestVerb<H> for FunctionVerb<H, F, ()>
where
    F: Fn(&mut H) + 'static,
    F: Clone,
{
    fn run(
        &self,
        harness: &mut H,
        node: &kdl::KdlNode,
        _arguments: &[kdl::KdlEntry],
    ) -> Result<TestRunResult, TestParseError> {
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (self.func)(harness);
        }));

        match res {
            Ok(()) => Ok(TestRunResult::Ok),
            Err(error) => {
                let mut message = "Something went wrong".to_string();

                let payload = error;

                if let Some(msg) = payload.downcast_ref::<&str>() {
                    message = msg.to_string();
                }

                if let Some(msg) = payload.downcast_ref::<String>() {
                    message.clone_from(msg);
                }

                Ok(TestRunResult::Error(TestRunResultError::Panic {
                    error: miette::Error::msg(message),
                    label: node.span(),
                }))
            }
        }
    }

    fn clone_box(&self) -> Box<dyn TestVerb<H>> {
        Box::new(self.clone())
    }
}

impl<F, H: 'static, P1> TestVerb<H> for FunctionVerb<H, F, (P1,)>
where
    F: Fn(&mut H, P1) + 'static,
    F: Clone,
    P1: VerbArgument,
{
    fn run(
        &self,
        harness: &mut H,
        node: &kdl::KdlNode,
        arguments: &[kdl::KdlEntry],
    ) -> Result<TestRunResult, TestParseError> {
        let arg = arguments
            .first()
            .ok_or_else(|| TestParseErrorCase::MissingArgument {
                parent: node.name().span(),
                missing: String::from("This node requires an integer argument"),
            })?;

        let arg: P1 =
            VerbArgument::from_value(arg).ok_or_else(|| TestParseErrorCase::WrongArgumentType {
                parent: node.name().span(),
                argument: arg.span(),
                expected: String::from("Expected an integer"),
            })?;

        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            (self.func)(harness, arg);
        }));

        match res {
            Ok(()) => Ok(TestRunResult::Ok),
            Err(error) => {
                let mut message = "Something went wrong".to_string();

                let payload = error;

                if let Some(msg) = payload.downcast_ref::<&str>() {
                    message = msg.to_string();
                }

                if let Some(msg) = payload.downcast_ref::<String>() {
                    message.clone_from(msg);
                }

                Ok(TestRunResult::Error(TestRunResultError::Panic {
                    error: miette::Error::msg(message),
                    label: node.span(),
                }))
            }
        }
    }

    fn clone_box(&self) -> Box<dyn TestVerb<H>> {
        Box::new(self.clone())
    }
}
