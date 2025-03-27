use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

pub use kdl;
use miette::Diagnostic;
use miette::NamedSource;
use thiserror::Error;

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
    func: F,
    _pd: PhantomData<fn(H, Args)>,
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

pub trait VerbArgument: Sized + 'static {
    fn from_value(value: &kdl::KdlEntry) -> Option<Self>;
}

impl VerbArgument for String {
    fn from_value(value: &kdl::KdlEntry) -> Option<Self> {
        value.value().as_string().map(ToOwned::to_owned)
    }
}

impl VerbArgument for usize {
    fn from_value(value: &kdl::KdlEntry) -> Option<Self> {
        value.value().as_integer().map(|i| i as usize)
    }
}

pub trait TestCondition<H>: 'static {}

pub struct TestDsl<H> {
    verbs: HashMap<String, Box<dyn TestVerb<H>>>,
    conditions: HashMap<String, Box<dyn TestCondition<H>>>,
}

impl<H> std::fmt::Debug for TestDsl<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestDsl").finish_non_exhaustive()
    }
}

impl<H: 'static> Default for TestDsl<H> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Error, Diagnostic, Debug)]
#[error("An error occurred while parsing testcases")]
pub struct TestParseError {
    #[related]
    errors: Vec<TestParseErrorCase>,

    #[source_code]
    source_code: Option<miette::NamedSource<Arc<str>>>,
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
enum TestParseErrorCase {
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

impl<H: 'static> TestDsl<H> {
    pub fn new() -> Self {
        TestDsl {
            verbs: HashMap::default(),
            conditions: HashMap::default(),
        }
    }

    pub fn add_verb(&mut self, name: impl AsRef<str>, verb: impl TestVerb<H>) {
        let existing = self.verbs.insert(name.as_ref().to_string(), Box::new(verb));
        assert!(existing.is_none());
    }

    pub fn add_conditions(&mut self, name: impl AsRef<str>, condition: impl TestCondition<H>) {
        let existing = self
            .conditions
            .insert(name.as_ref().to_string(), Box::new(condition));

        assert!(existing.is_none());
    }

    pub fn parse_document(
        &self,
        input: miette::NamedSource<Arc<str>>,
    ) -> Result<Vec<TestCase<H>>, TestParseError> {
        let document = kdl::KdlDocument::parse(input.inner())?;

        let mut cases = vec![];

        let mut errors = vec![];

        for testcase_node in document.nodes() {
            if testcase_node.name().value() != "testcase" {
                errors.push(TestParseErrorCase::NotTestcase {
                    span: testcase_node.name().span(),
                });

                continue;
            }

            let mut testcase = TestCase::new(input.clone());

            for verb_node in testcase_node.iter_children() {
                match self.parse_node(verb_node) {
                    Ok(verb) => testcase.creators.push(verb),
                    Err(e) => errors.push(e),
                }
            }

            cases.push(testcase);
        }

        if !errors.is_empty() {
            return Err(TestParseError {
                errors,
                source_code: Some(input.clone()),
            });
        }

        Ok(cases)
    }

    fn parse_node(
        &self,
        verb_node: &kdl::KdlNode,
    ) -> Result<Box<dyn TestVerbCreator<H>>, TestParseErrorCase> {
        match verb_node.name().value() {
            "repeat" => {
                let times = verb_node
                    .get(0)
                    .ok_or_else(|| TestParseErrorCase::MissingArgument {
                        parent: verb_node.name().span(),
                        missing: String::from("`repeat` takes one argument, the repetition count"),
                    })?
                    .as_integer()
                    .ok_or_else(|| TestParseErrorCase::WrongArgumentType {
                        parent: verb_node.name().span(),
                        argument: verb_node.iter().next().unwrap().span(),
                        expected: String::from("Expected an integer"),
                    })? as usize;

                let block = verb_node
                    .iter_children()
                    .map(|node| self.parse_node(node))
                    .collect::<Result<_, _>>()?;

                Ok(Box::new(Repeat { times, block }))
            }
            "group" => Ok(Box::new(Group {
                block: verb_node
                    .iter_children()
                    .map(|n| self.parse_node(n))
                    .collect::<Result<_, _>>()?,
            })),
            name => {
                let verb = self
                    .verbs
                    .get(name)
                    .ok_or_else(|| TestParseErrorCase::UnknownVerb {
                        verb: verb_node.name().span(),
                    })?
                    .clone();
                let params = verb_node.iter().cloned().collect();

                Ok(Box::new(Identity {
                    verb,
                    node: verb_node.clone(),
                    params,
                }))
            }
        }
    }
}

struct Group<H> {
    block: Vec<Box<dyn TestVerbCreator<H>>>,
}

impl<H: 'static> TestVerbCreator<H> for Group<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = TestVerbInstance<'_, H>> + '_> {
        Box::new(self.block.iter().flat_map(|c| c.get_test_verbs()))
    }
}

struct Repeat<H> {
    times: usize,
    block: Vec<Box<dyn TestVerbCreator<H>>>,
}

impl<H: 'static> TestVerbCreator<H> for Repeat<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = TestVerbInstance<'_, H>> + '_> {
        Box::new(
            std::iter::repeat_with(|| self.block.iter().flat_map(|c| c.get_test_verbs()))
                .take(self.times)
                .flatten(),
        )
    }
}

struct Identity<H> {
    verb: Box<dyn TestVerb<H>>,
    node: kdl::KdlNode,
    params: Vec<kdl::KdlEntry>,
}

impl<H: 'static> TestVerbCreator<H> for Identity<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = TestVerbInstance<'_, H>> + '_> {
        Box::new(std::iter::once(TestVerbInstance {
            verb: self.verb.clone(),
            node: &self.node,
            params: &self.params,
        }))
    }
}

struct TestVerbInstance<'p, H> {
    verb: Box<dyn TestVerb<H>>,
    node: &'p kdl::KdlNode,
    params: &'p [kdl::KdlEntry],
}

impl<'p, H: 'static> TestVerbInstance<'p, H> {
    fn run<'h>(&'p self, harness: &'h mut H) -> Result<TestRunResult, TestParseError> {
        self.verb.run(harness, self.node, self.params)
    }
}

trait TestVerbCreator<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = TestVerbInstance<'_, H>> + '_>;
}

pub struct TestCase<H> {
    creators: Vec<Box<dyn TestVerbCreator<H>>>,
    source_code: NamedSource<Arc<str>>,
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
    error: TestCaseErrorCase,

    #[source_code]
    source_code: miette::NamedSource<Arc<str>>,
}

#[derive(Error, Diagnostic, Debug)]
enum TestCaseErrorCase {
    #[diagnostic(transparent)]
    #[error("Failed while running")]
    Run { error: TestRunResultError },
    #[diagnostic(transparent)]
    #[error("Failed while parsing")]
    Parse { error: TestParseError },
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
                    .map(|verb| match verb.run(harness) {
                        Ok(TestRunResult::Ok) => Ok(()),
                        Ok(TestRunResult::Error(error)) => Err(TestCaseErrorCase::Run { error }),
                        Err(error) => Err(TestCaseErrorCase::Parse { error }),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Result<(), _>>()
            .map_err(|error| TestCaseError {
                error,
                source_code: self.source_code.clone(),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    use miette::NamedSource;

    use crate::FunctionVerb;
    use crate::TestDsl;

    struct ArithmeticHarness {
        value: AtomicUsize,
    }

    #[test]
    fn simple_test() {
        let mut ts = TestDsl::<ArithmeticHarness>::new();
        ts.add_verb(
            "add_one",
            FunctionVerb::from(|ah: &mut ArithmeticHarness| {
                ah.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        ts.add_verb(
            "mul_two",
            FunctionVerb::from(|ah: &mut ArithmeticHarness| {
                let value = ah.value.load(std::sync::atomic::Ordering::SeqCst);
                ah.value
                    .store(value * 2, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        let tc = ts
            .parse_document(NamedSource::new(
                "test.kdl",
                Arc::from(
                    r#"
            testcase {
                add_one
                add_one
                mul_two
            }
            "#,
                ),
            ))
            .unwrap();

        let mut ah = ArithmeticHarness {
            value: AtomicUsize::new(0),
        };

        tc[0].run(&mut ah).unwrap();

        assert_eq!(ah.value.load(std::sync::atomic::Ordering::SeqCst), 4);
    }

    #[test]
    fn repeat_test() {
        let mut ts = TestDsl::<ArithmeticHarness>::new();
        ts.add_verb(
            "add_one",
            FunctionVerb::from(|ah: &mut ArithmeticHarness| {
                ah.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        ts.add_verb(
            "mul_two",
            FunctionVerb::from(|ah: &mut ArithmeticHarness| {
                let value = ah.value.load(std::sync::atomic::Ordering::SeqCst);
                ah.value
                    .store(value * 2, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        let tc = ts
            .parse_document(NamedSource::new(
                "test.kdl",
                Arc::from(
                    r#"
            testcase {
                repeat 2 {
                    repeat 2 {
                        add_one
                        mul_two
                    }
                }
            }
            "#,
                ),
            ))
            .unwrap();

        let mut ah = ArithmeticHarness {
            value: AtomicUsize::new(0),
        };

        tc[0].run(&mut ah).unwrap();

        assert_eq!(ah.value.load(std::sync::atomic::Ordering::SeqCst), 30);
    }

    #[test]
    fn check_arguments_work() {
        let mut ts = TestDsl::<ArithmeticHarness>::new();
        ts.add_verb(
            "add_one",
            FunctionVerb::from(|ah: &mut ArithmeticHarness| {
                ah.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        ts.add_verb(
            "add",
            FunctionVerb::from(|ah: &mut ArithmeticHarness, num: usize| {
                ah.value.fetch_add(num, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        ts.add_verb(
            "mul_two",
            FunctionVerb::from(|ah: &mut ArithmeticHarness| {
                let value = ah.value.load(std::sync::atomic::Ordering::SeqCst);
                ah.value
                    .store(value * 2, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        let tc = ts
            .parse_document(NamedSource::new(
                "test.kdl",
                Arc::from(
                    r#"
            testcase {
                repeat 2 {
                    repeat 2 {
                        group {
                            add 2
                            mul_two
                        }
                    }
                }
            }
            "#,
                ),
            ))
            .unwrap();

        let mut ah = ArithmeticHarness {
            value: AtomicUsize::new(0),
        };

        tc[0].run(&mut ah).unwrap();

        assert_eq!(ah.value.load(std::sync::atomic::Ordering::SeqCst), 60);
    }
}
