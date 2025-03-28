//! Common error definitions
use miette::Diagnostic;
use thiserror::Error;

use crate::TestCaseInput;

#[derive(Error, Diagnostic, Debug)]
#[error("An error occurred while parsing testcases")]
/// An error occurred while parsing testcases
pub struct TestParseError {
    #[related]
    pub(crate) errors: Vec<TestErrorCase>,

    #[source_code]
    pub(crate) source_code: Option<TestCaseInput>,
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
/// Errors that can happen related to tests
pub enum TestErrorCase {
    #[error("An error occurred while parsing the KDL data")]
    /// KDL reported an error
    Kdl {
        #[source]
        /// The specific KDL error
        source: kdl::KdlError,
    },
    #[error("Not a valid test case")]
    #[diagnostic(help("The outer items must all be `testcase`s"))]
    /// An outer node was not a `testcase` node
    NotTestcase {
        #[label("Expected a `testcase`")]
        /// The location of the offending node
        span: miette::SourceSpan,
    },
    #[error("An argument was missing")]
    /// An argument was missing from a node
    MissingArgument {
        #[label("This node is missing an argument")]
        /// The location of the node
        parent: miette::SourceSpan,

        #[help]
        /// Help related to what was missing
        missing: String,
    },
    #[error("An argument was of the wrong type")]
    /// A node had a wrong type in its parameter list
    WrongArgumentType {
        #[label("This node has an argument of a wrong kind")]
        /// The parent node
        parent: miette::SourceSpan,

        #[label("this one")]
        /// The offending argument
        argument: miette::SourceSpan,

        #[help]
        /// Help text to explain what was expected, if possible
        expected: String,
    },
    #[error("Could not find condition with this name")]
    /// The given condition could not be found
    UnknownCondition {
        #[label]
        /// The location of the condition node
        condition: miette::SourceSpan,
    },
    #[error("Could not find verb with this name")]
    /// The given verb could not be found
    UnknownVerb {
        #[label]
        /// The location of the verb node
        verb: miette::SourceSpan,
    },

    #[error("A panic occurred while running the test")]
    /// An panic occurred while running a test
    Panic {
        #[diagnostic_source]
        /// The message of the panic if it had one
        error: miette::Error,

        #[label("in this node")]
        /// Which node caused the panic
        label: miette::SourceSpan,
    },

    #[error("An error occurred while running the test")]
    /// An error occurred in a node
    Error {
        #[diagnostic_source]
        /// The error as it was given
        error: miette::Error,

        #[label("in this node")]
        /// Which node cause the error
        label: miette::SourceSpan,
    },
}
