use std::collections::HashMap;
use std::marker::PhantomData;

use miette::Diagnostic;
use thiserror::Error;

pub trait TestVerb<H>: 'static {
    fn run(&self, harness: &H, arguments: &[kdl::KdlValue]);
    fn clone_box(&self) -> Box<dyn TestVerb<H>>;
}

impl<H: 'static> Clone for Box<dyn TestVerb<H>> {
    fn clone(&self) -> Self {
        let this: &dyn TestVerb<H> = &**self;
        this.clone_box()
    }
}

struct FunctionVerb<H, F, Args> {
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
    F: Fn(&H),
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
    F: Fn(&H, usize),
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
    F: Fn(&H) + 'static,
    F: Clone,
{
    fn run(&self, harness: &H, _arguments: &[kdl::KdlValue]) {
        (self.func)(harness)
    }

    fn clone_box(&self) -> Box<dyn TestVerb<H>> {
        Box::new(self.clone())
    }
}

impl<F, H: 'static> TestVerb<H> for FunctionVerb<H, F, (usize,)>
where
    F: Fn(&H, usize) + 'static,
    F: Clone,
{
    fn run(&self, harness: &H, arguments: &[kdl::KdlValue]) {
        (self.func)(harness, VerbArgument::from_value(&arguments[0]).unwrap())
    }

    fn clone_box(&self) -> Box<dyn TestVerb<H>> {
        Box::new(self.clone())
    }
}

pub trait VerbArgument: Sized {
    fn from_value(value: &kdl::KdlValue) -> Option<Self>;
}

impl VerbArgument for String {
    fn from_value(value: &kdl::KdlValue) -> Option<Self> {
        value.as_string().map(ToOwned::to_owned)
    }
}

impl VerbArgument for usize {
    fn from_value(value: &kdl::KdlValue) -> Option<Self> {
        value.as_integer().map(|i| i as usize)
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
    source_code: Option<String>,
}

impl From<kdl::KdlError> for TestParseError {
    fn from(source: kdl::KdlError) -> Self {
        TestParseError {
            errors: vec![TestParseErrorCase::Kdl { source }],
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

    pub fn parse_document(&self, input: &str) -> Result<Vec<TestCase<H>>, TestParseError> {
        let document = kdl::KdlDocument::parse(input)?;

        let mut cases = vec![];

        let mut errors = vec![];

        for testcase_node in document.nodes() {
            if testcase_node.name().value() != "testcase" {
                errors.push(TestParseErrorCase::NotTestcase {
                    span: testcase_node.name().span(),
                });

                continue;
            }

            let mut testcase = TestCase::new();

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
                source_code: Some(input.to_string()),
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
                let params = verb_node.iter().map(|e| e.value().clone()).collect();

                Ok(Box::new(Identity { verb, params }))
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
    params: Vec<kdl::KdlValue>,
}

impl<H: 'static> TestVerbCreator<H> for Identity<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = TestVerbInstance<'_, H>> + '_> {
        Box::new(std::iter::once(TestVerbInstance {
            verb: self.verb.clone(),
            params: &self.params,
        }))
    }
}

struct TestVerbInstance<'p, H> {
    verb: Box<dyn TestVerb<H>>,
    params: &'p [kdl::KdlValue],
}

impl<H: 'static> TestVerbInstance<'_, H> {
    fn run(&self, harness: &H) {
        self.verb.run(harness, self.params);
    }
}

trait TestVerbCreator<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = TestVerbInstance<'_, H>> + '_>;
}

pub struct TestCase<H> {
    creators: Vec<Box<dyn TestVerbCreator<H>>>,
}

impl<H> std::fmt::Debug for TestCase<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestCase").finish_non_exhaustive()
    }
}

impl<H: 'static> Default for TestCase<H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<H: 'static> TestCase<H> {
    pub fn new() -> Self {
        TestCase { creators: vec![] }
    }

    pub fn run(&self, harness: &H) {
        for c in &self.creators {
            for verb in c.get_test_verbs() {
                verb.run(harness);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

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
            FunctionVerb::from(|ah: &ArithmeticHarness| {
                ah.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        ts.add_verb(
            "mul_two",
            FunctionVerb::from(|ah: &ArithmeticHarness| {
                let value = ah.value.load(std::sync::atomic::Ordering::SeqCst);
                ah.value
                    .store(value * 2, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        let tc = ts
            .parse_document(
                r#"
            testcase {
                add_one
                add_one
                mul_two
            }
            "#,
            )
            .unwrap();

        let ah = ArithmeticHarness {
            value: AtomicUsize::new(0),
        };

        tc[0].run(&ah);

        assert_eq!(ah.value.load(std::sync::atomic::Ordering::SeqCst), 4);
    }

    #[test]
    fn repeat_test() {
        let mut ts = TestDsl::<ArithmeticHarness>::new();
        ts.add_verb(
            "add_one",
            FunctionVerb::from(|ah: &ArithmeticHarness| {
                ah.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        ts.add_verb(
            "mul_two",
            FunctionVerb::from(|ah: &ArithmeticHarness| {
                let value = ah.value.load(std::sync::atomic::Ordering::SeqCst);
                ah.value
                    .store(value * 2, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        let tc = ts
            .parse_document(
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
            )
            .unwrap();

        let ah = ArithmeticHarness {
            value: AtomicUsize::new(0),
        };

        tc[0].run(&ah);

        assert_eq!(ah.value.load(std::sync::atomic::Ordering::SeqCst), 30);
    }

    #[test]
    fn check_arguments_work() {
        let mut ts = TestDsl::<ArithmeticHarness>::new();
        ts.add_verb(
            "add_one",
            FunctionVerb::from(|ah: &ArithmeticHarness| {
                ah.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        ts.add_verb(
            "add",
            FunctionVerb::from(|ah: &ArithmeticHarness, num: usize| {
                ah.value.fetch_add(num, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        ts.add_verb(
            "mul_two",
            FunctionVerb::from(|ah: &ArithmeticHarness| {
                let value = ah.value.load(std::sync::atomic::Ordering::SeqCst);
                ah.value
                    .store(value * 2, std::sync::atomic::Ordering::SeqCst);
            }),
        );

        let tc = ts
            .parse_document(
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
            )
            .unwrap();

        let ah = ArithmeticHarness {
            value: AtomicUsize::new(0),
        };

        tc[0].run(&ah);

        assert_eq!(ah.value.load(std::sync::atomic::Ordering::SeqCst), 60);
    }
}
