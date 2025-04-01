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
    /// KDL reported an error
    #[error("An error occurred while parsing the KDL data")]
    Kdl {
        /// The specific KDL error
        #[source]
        source: kdl::KdlError,
    },

    /// An outer node was not a `testcase` node
    #[error("Not a valid test case")]
    #[diagnostic(help("The outer items must all be `testcase`s"))]
    NotTestcase {
        /// The location of the offending node
        #[label("Expected a `testcase`")]
        span: miette::SourceSpan,
    },

    /// An argument was missing from a node
    #[error("An argument was missing")]
    MissingArgument {
        /// The location of the node
        #[label("This node is missing an argument")]
        parent: miette::SourceSpan,

        /// Help related to what was missing
        #[help]
        missing: String,
    },

    /// A node had a wrong type in its parameter list
    #[error("An argument was of the wrong type")]
    WrongArgumentType {
        /// The parent node
        #[label("This node has an argument of a wrong kind")]
        parent: miette::SourceSpan,

        /// The offending argument
        #[label("this one")]
        argument: miette::SourceSpan,

        /// Help text to explain what was expected, if possible
        #[help]
        expected: String,
    },

    /// The given condition could not be found
    #[error("Could not find condition with this name")]
    UnknownCondition {
        /// The location of the condition node
        #[label]
        condition: miette::SourceSpan,
    },

    /// The given verb could not be found
    #[error("Could not find verb with this name")]
    UnknownVerb {
        /// The location of the verb node
        #[label]
        verb: miette::SourceSpan,
    },

    /// The condition is not valid in this position
    #[error("The condition is not valid in this position")]
    InvalidCondition {
        /// The inner error
        #[diagnostic_source]
        error: miette::Error,
    },
}

#[derive(Debug, Error, Diagnostic)]
/// Errors occurring while running tests
pub enum TestError {
    /// An error occurred in a verb/condition
    #[error("An error occurred")]
    Error {
        #[diagnostic_source]
        /// The returned error
        error: miette::Error,

        #[label("in this node")]
        /// Which node caused the panic
        span: miette::SourceSpan,
    },

    /// An panic occurred in a verb/condition
    #[error("A panic occurred")]
    Panic {
        #[diagnostic_source]
        /// The message of the panic if it had one
        error: miette::Error,

        #[label("in this node")]
        /// Which node caused the panic
        span: miette::SourceSpan,
    },

    /// The evaluated condition failed
    #[error("The given condition failed")]
    ConditionFailed {
        #[label("in this node")]
        /// Which node caused the panic
        span: miette::SourceSpan,
    },
}
