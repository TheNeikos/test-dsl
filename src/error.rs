use std::sync::Arc;

use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
#[error("An error occurred while parsing testcases")]
pub struct TestParseError {
    #[related]
    pub(crate) errors: Vec<TestErrorCase>,

    #[source_code]
    pub(crate) source_code: Option<miette::NamedSource<Arc<str>>>,
}

impl From<kdl::KdlError> for TestParseError {
    fn from(source: kdl::KdlError) -> Self {
        TestParseError {
            errors: vec![TestErrorCase::Kdl { source }],
            source_code: None,
        }
    }
}

impl From<TestErrorCase> for TestParseError {
    fn from(source: TestErrorCase) -> Self {
        TestParseError {
            errors: vec![source],
            source_code: None,
        }
    }
}

#[derive(Error, Diagnostic, Debug)]
pub enum TestErrorCase {
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
    #[error("Could not find condition with this name")]
    UnknownCondition {
        #[label]
        condition: miette::SourceSpan,
    },
    #[error("Could not find verb with this name")]
    UnknownVerb {
        #[label]
        verb: miette::SourceSpan,
    },

    #[error("A panic occurred while running the test")]
    Panic {
        #[diagnostic_source]
        error: miette::Error,

        #[label("in this node")]
        label: miette::SourceSpan,
    },

    #[error("An error occurred while running the test")]
    Error {
        #[diagnostic_source]
        error: miette::Error,

        #[label("in this node")]
        label: miette::SourceSpan,
    },
}
