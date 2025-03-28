use crate::error::TestParseError;
use crate::error::TestRunResult;

pub trait TestCondition<H>: 'static {
    fn run(
        &self,
        harness: &mut H,
        node: &kdl::KdlNode,
    ) -> Result<TestRunResult, TestParseError>;
    fn clone_box(&self) -> Box<dyn TestCondition<H>>;
}
