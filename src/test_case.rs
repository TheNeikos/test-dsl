use std::sync::Arc;

use miette::Diagnostic;
use miette::NamedSource;
use thiserror::Error;

use super::TestVerbCreator;
use crate::error::TestRunResultError;

pub struct TestCase<H> {
    pub(crate) creators: Vec<Box<dyn TestVerbCreator<H>>>,
    pub(crate) source_code: NamedSource<Arc<str>>,
}

impl<H> std::fmt::Debug for TestCase<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestCase").finish_non_exhaustive()
    }
}

#[derive(Error, Diagnostic, Debug)]
#[error("Testcase did not run successfully")]
pub struct TestCaseError {
    #[diagnostic_source]
    pub(crate) error: TestRunResultError,

    #[source_code]
    pub(crate) source_code: miette::NamedSource<Arc<str>>,
}

impl<H: 'static> TestCase<H> {
    pub fn new(source_code: miette::NamedSource<Arc<str>>) -> Self {
        TestCase {
            creators: vec![],
            source_code,
        }
    }

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
