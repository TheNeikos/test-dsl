//! Individual testcases

use miette::Diagnostic;
use thiserror::Error;

use crate::TestCaseInput;
use crate::VerbInstance;
use crate::error::TestErrorCase;

/// A singular test case
pub struct TestCase<H> {
    pub(crate) cases: Vec<VerbInstance<H>>,
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
            cases: vec![],
            source_code,
        }
    }

    /// Get the path of the source of this test case.
    ///
    /// Returns `None` if the test case source came from in-memory.
    pub fn path(&self) -> Option<&str> {
        match self.source_code {
            TestCaseInput::InMemory(_) => None,
            TestCaseInput::FromFile { ref filepath, .. } => Some(&**filepath),
        }
    }

    /// Run the given test and report on its success
    pub fn run(&self, harness: &mut H) -> Result<(), TestCaseError> {
        self.cases
            .iter()
            .try_for_each(|verb| verb.run(harness))
            .map_err(|error| TestCaseError {
                error,
                source_code: self.source_code.clone(),
            })
    }
}
