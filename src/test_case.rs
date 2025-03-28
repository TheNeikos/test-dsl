//! Individual testcases

use miette::Diagnostic;
use thiserror::Error;

use super::TestVerbCreator;
use crate::TestCaseInput;
use crate::error::TestErrorCase;

/// A singular test case
pub struct TestCase<H> {
    pub(crate) creators: Vec<Box<dyn TestVerbCreator<H>>>,
    pub(crate) source_code: TestCaseInput,
}

impl<H> std::fmt::Debug for TestCase<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestCase").finish_non_exhaustive()
    }
}

#[derive(Error, Diagnostic, Debug)]
#[error("Testcase did not run successfully")]
/// An error occured while running a test
pub struct TestCaseError {
    #[diagnostic_source]
    pub(crate) error: TestErrorCase,

    #[source_code]
    pub(crate) source_code: TestCaseInput,
}

impl<H: 'static> TestCase<H> {
    pub(crate) fn new(source_code: TestCaseInput) -> Self {
        TestCase {
            creators: vec![],
            source_code,
        }
    }

    /// Run the given test and report on its success
    pub fn run(&self, harness: &mut H) -> Result<(), TestCaseError> {
        self.creators
            .iter()
            .flat_map(|c| {
                c.get_test_verbs()
                    .map(|verb| verb.run(harness))
                    .collect::<Vec<_>>()
            })
            .collect::<Result<(), _>>()
            .map_err(|error| TestCaseError {
                error,
                source_code: self.source_code.clone(),
            })
    }
}
