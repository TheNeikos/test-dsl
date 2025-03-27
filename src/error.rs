use std::sync::Arc;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
#[error("An error occurred while parsing testcases")]
pub struct TestParseError {
    #[related]
    pub(crate) errors: Vec<TestParseErrorCase>,

    #[source_code]
    pub(crate) source_code: Option<miette::NamedSource<Arc<str>>>,
}

impl From<kdl::KdlError> for TestParseError {
    fn from(source: kdl::KdlError) -> Self {
        TestParseError {
            errors: vec![TestParseErrorCase::Kdl { source }],
            source_code: None,
        }
    }
}

impl From<TestParseErrorCase> for TestParseError {
    fn from(source: TestParseErrorCase) -> Self {
        TestParseError {
            errors: vec![source],
            source_code: None,
        }
    }
}

#[derive(Error, Diagnostic, Debug)]
pub(crate) enum TestParseErrorCase {
    #[error("An error occurred while parsing the KDL data")]
    Kdl {
        #[source]
        source: kdl::KdlError,
    },
    #[error("Not a valid test case")]
    #[diagnostic(help("The outer items must all be `testcase`s"))]
    NotTestcase {
        #[label("Expected a `testcase`")]
        span: miette::SourceSpan,
    },
    #[error("An argument was missing")]
    MissingArgument {
        #[label("This node is missing an argument")]
        parent: miette::SourceSpan,

        #[help]
        missing: String,
    },
    #[error("An argument was of the wrong type")]
    WrongArgumentType {
        #[label("This node has an argument of a wrong kind")]
        parent: miette::SourceSpan,

        #[label("this one")]
        argument: miette::SourceSpan,

        #[help]
        expected: String,
    },
    #[error("Could not find verb with this name")]
    UnknownVerb {
        #[label]
        verb: miette::SourceSpan,
    },
}

pub enum TestRunResult {
    Ok,
    Error(TestRunResultError),
}

#[derive(Error, Diagnostic, Debug)]
#[error("An error occurred while running the test")]
pub enum TestRunResultError {
    Panic {
        #[diagnostic_source]
        error: miette::Error,

        #[label("in this node")]
        label: miette::SourceSpan,
    },

    Error {
        #[diagnostic_source]
        error: miette::Error,

        #[label("in this node")]
        label: miette::SourceSpan,
    },
}

