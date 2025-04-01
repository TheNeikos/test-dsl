#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use arguments::ParseArguments;
use condition::ErasedCondition;
use error::TestErrorCase;
use verb::ErasedVerb;
use verb::Verb;

#[macro_use]
mod macros;

pub mod arguments;
pub mod condition;
pub mod error;
pub mod test_case;
pub mod verb;
pub use kdl;
pub use miette;

/// The main type of the crate
///
/// It contains all available verbs and conditions, and is used to derive
/// [`TestCase`](test_case::TestCase)s.
pub struct TestDsl<H> {
    verbs: HashMap<String, ErasedVerb<H>>,
    conditions: HashMap<String, ErasedCondition<H>>,
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

impl<H: 'static> TestDsl<H> {
    /// Create an empty [`TestDsl`]
    pub fn new() -> Self {
        let mut dsl = TestDsl {
            verbs: HashMap::default(),
            conditions: HashMap::default(),
        };

        dsl.add_verb("repeat", Repeat);
        dsl.add_verb("group", Group);
        dsl.add_verb("assert", AssertConditions);

        dsl
    }

    /// Add a single verb
    ///
    /// The name is used as-is in your testcases, the arguments are up to each individual [`TestVerb`] implementation.
    ///
    /// See [`FunctionVerb`](verb::FunctionVerb) for an easy to use way of defining verbs.
    pub fn add_verb(&mut self, name: impl AsRef<str>, verb: impl Verb<H>) {
        let existing = self
            .verbs
            .insert(name.as_ref().to_string(), ErasedVerb::erase(verb));
        assert!(existing.is_none());
    }

    /// Add a single condition
    ///
    /// The name is used as-is in your testcases, the arguments are up to each individual
    /// [`TestCondition`] implementation.
    ///
    /// See [`Condition`](condition::Condition) for an easy to use way of defining conditions.
    pub fn add_condition(
        &mut self,
        name: impl AsRef<str>,
        condition: impl condition::TestCondition<H>,
    ) {
        let existing = self
            .conditions
            .insert(name.as_ref().to_string(), ErasedCondition::erase(condition));

        assert!(existing.is_none());
    }

    /// Parse a given document as a [`KdlDocument`](kdl::KdlDocument) and generate a
    /// [`TestCase`](test_case::TestCase) out of it.
    pub fn parse_testcase(
        &self,
        input: impl Into<TestCaseInput>,
    ) -> Result<Vec<test_case::TestCase<H>>, error::TestParseError> {
        let input = input.into();
        let document = kdl::KdlDocument::parse(input.content())?;

        let mut cases = vec![];

        let mut errors = vec![];

        for testcase_node in document.nodes() {
            if testcase_node.name().value() != "testcase" {
                errors.push(error::TestErrorCase::NotTestcase {
                    span: testcase_node.name().span(),
                });

                continue;
            }

            let mut testcase = test_case::TestCase::new(input.clone());

            for node in testcase_node.iter_children() {
                match VerbInstance::with_test_dsl(self, node) {
                    Ok(verb) => testcase.cases.push(verb),
                    Err(e) => errors.push(e),
                }
            }

            cases.push(testcase);
        }

        if !errors.is_empty() {
            return Err(error::TestParseError {
                errors,
                source_code: Some(input.clone()),
            });
        }

        Ok(cases)
    }

    fn get_condition_for_node(
        &self,
        condition_node: &kdl::KdlNode,
    ) -> Result<ErasedCondition<H>, error::TestErrorCase> {
        let condition = self
            .conditions
            .get(condition_node.name().value())
            .ok_or_else(|| error::TestErrorCase::UnknownCondition {
                condition: condition_node.name().span(),
            })?
            .clone();

        Ok(condition)
    }

    fn get_verb_for_node(
        &self,
        verb_node: &kdl::KdlNode,
    ) -> Result<ErasedVerb<H>, error::TestErrorCase> {
        let verb = self
            .verbs
            .get(verb_node.name().value())
            .ok_or_else(|| error::TestErrorCase::UnknownVerb {
                verb: verb_node.name().span(),
            })?
            .clone();

        Ok(verb)
    }
}

#[derive(Debug, Clone)]
/// The input to a [`TestCase`](test_case::TestCase)
pub enum TestCaseInput {
    /// Input that is not backed by a file
    InMemory(Arc<str>),
    /// Input that is backed by a file
    FromFile {
        /// The filepath of the file the contents are read from
        filepath: Arc<str>,
        /// The content of the file
        contents: Arc<str>,
    },
}

impl From<&str> for TestCaseInput {
    fn from(value: &str) -> Self {
        TestCaseInput::InMemory(Arc::from(value))
    }
}

impl miette::SourceCode for TestCaseInput {
    fn read_span<'a>(
        &'a self,
        span: &miette::SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
        match self {
            TestCaseInput::InMemory(content) => {
                content.read_span(span, context_lines_before, context_lines_after)
            }
            TestCaseInput::FromFile {
                filepath: filename,
                contents,
            } => {
                let inner_contents =
                    contents.read_span(span, context_lines_before, context_lines_after)?;
                let mut contents = miette::MietteSpanContents::new_named(
                    filename.to_string(),
                    inner_contents.data(),
                    *inner_contents.span(),
                    inner_contents.line(),
                    inner_contents.column(),
                    inner_contents.line_count(),
                );
                contents = contents.with_language("kdl");
                Ok(Box::new(contents))
            }
        }
    }
}

impl TestCaseInput {
    fn content(&self) -> &str {
        match self {
            TestCaseInput::InMemory(content) => content,
            TestCaseInput::FromFile { contents, .. } => contents,
        }
    }
}

#[derive(Debug, Clone)]
struct AssertConditions;

impl<H: 'static> Verb<H> for AssertConditions {
    type Arguments = ConditionChildren<H, ((),)>;
    fn run(&self, harness: &mut H, arguments: &Self::Arguments) -> Result<(), TestErrorCase> {
        for child in arguments.children() {
            child.run(harness)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Group;

impl<H: 'static> Verb<H> for Group {
    type Arguments = VerbChildren<H, ((),)>;
    fn run(
        &self,
        harness: &mut H,
        arguments: &Self::Arguments,
    ) -> Result<(), error::TestErrorCase> {
        for child in arguments.children() {
            child.run(harness)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Repeat;

impl<H: 'static> Verb<H> for Repeat {
    type Arguments = VerbChildren<H, (usize,)>;
    fn run(
        &self,
        harness: &mut H,
        arguments: &Self::Arguments,
    ) -> Result<(), error::TestErrorCase> {
        let (times,) = *arguments.parameters();

        for _ in 0..times {
            for child in arguments.children() {
                child.run(harness)?;
            }
        }

        Ok(())
    }
}

/// Parameters with a list of nodes that are conditions
pub struct ConditionChildren<H, A> {
    parameters: A,
    children: Vec<ConditionInstance<H>>,
}

impl<H, A> ConditionChildren<H, A> {
    /// Get the parameters
    pub fn parameters(&self) -> &A {
        &self.parameters
    }

    /// Get the children nodes
    pub fn children(&self) -> &[ConditionInstance<H>] {
        &self.children
    }
}

impl<H: 'static, A: ParseArguments<H>> ParseArguments<H> for ConditionChildren<H, A> {
    fn parse(test_dsl: &TestDsl<H>, node: &kdl::KdlNode) -> Result<Self, error::TestErrorCase> {
        let arguments = A::parse(test_dsl, node)?;

        let children = node
            .iter_children()
            .map(|node| ConditionInstance::with_test_dsl(test_dsl, node))
            .collect::<Result<_, _>>()?;

        Ok(ConditionChildren {
            parameters: arguments,
            children,
        })
    }
}

/// Parameters with a list of nodes that are verbs
pub struct VerbChildren<H, A> {
    parameters: A,
    children: Vec<VerbInstance<H>>,
}

impl<H, A> VerbChildren<H, A> {
    /// Get the parameters
    pub fn parameters(&self) -> &A {
        &self.parameters
    }

    /// Get the children
    pub fn children(&self) -> &[VerbInstance<H>] {
        &self.children
    }
}

impl<H: 'static, A: ParseArguments<H>> ParseArguments<H> for VerbChildren<H, A> {
    fn parse(test_dsl: &TestDsl<H>, node: &kdl::KdlNode) -> Result<Self, error::TestErrorCase> {
        let arguments = A::parse(test_dsl, node)?;

        let children = node
            .iter_children()
            .map(|node| VerbInstance::with_test_dsl(test_dsl, node))
            .collect::<Result<_, _>>()?;

        Ok(VerbChildren {
            parameters: arguments,
            children,
        })
    }
}

/// An instance of a [`TestCondition`](condition::TestCondition)
pub struct ConditionInstance<H> {
    _pd: PhantomData<fn(H)>,
    condition: ErasedCondition<H>,
    arguments: Box<dyn std::any::Any>,
}

impl<H: 'static> ConditionInstance<H> {
    /// Create a new instance with the given node and [`TestDsl`]
    pub fn with_test_dsl(
        test_dsl: &TestDsl<H>,
        node: &kdl::KdlNode,
    ) -> Result<Self, TestErrorCase> {
        let condition = test_dsl.get_condition_for_node(node)?;

        let arguments = condition.parse_args(test_dsl, node)?;

        Ok(ConditionInstance {
            _pd: PhantomData,
            condition,
            arguments,
        })
    }

    /// Run the condition
    ///
    /// This returns an error if:
    /// - The condition returns [`Ok(false)`](Ok)
    /// - It returns an [`Err`]
    /// - It [`panic`]s
    pub fn run(&self, harness: &mut H) -> Result<(), TestErrorCase> {
        self.condition
            .check_now(harness, &*self.arguments)
            .and_then(|res| res.then_some(()).ok_or(TestErrorCase::ConditionFailed))
    }
}

/// An instance of a [`Verb`]
pub struct VerbInstance<H> {
    _pd: PhantomData<fn(H)>,
    verb: ErasedVerb<H>,
    arguments: Box<dyn std::any::Any>,
}

impl<H: 'static> VerbInstance<H> {
    /// Create a new instance with the given node and [`TestDsl`]
    pub fn with_test_dsl(
        test_dsl: &TestDsl<H>,
        node: &kdl::KdlNode,
    ) -> Result<Self, TestErrorCase> {
        let verb = test_dsl.get_verb_for_node(node)?;

        let arguments = verb.parse_args(test_dsl, node)?;

        Ok(VerbInstance {
            _pd: PhantomData,
            verb,
            arguments,
        })
    }

    /// Run the verb
    ///
    /// This returns an error if:
    /// - It returns an [`Err`]
    /// - It [`panic`]s
    pub fn run(&self, harness: &mut H) -> Result<(), TestErrorCase> {
        self.verb.run(harness, &*self.arguments)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use crate::TestDsl;
    use crate::verb::FunctionVerb;

    struct ArithmeticHarness {
        value: AtomicUsize,
    }

    #[test]
    fn simple_test() {
        let mut ts = TestDsl::<ArithmeticHarness>::new();
        ts.add_verb(
            "add_one",
            FunctionVerb::new(|ah: &mut ArithmeticHarness| {
                ah.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                Ok(())
            }),
        );

        ts.add_verb(
            "mul_two",
            FunctionVerb::new(|ah: &mut ArithmeticHarness| {
                let value = ah.value.load(std::sync::atomic::Ordering::SeqCst);
                ah.value
                    .store(value * 2, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }),
        );

        let tc = ts
            .parse_testcase(
                r#"
            testcase {
                add_one
                add_one
                mul_two
            }
            "#,
            )
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
            FunctionVerb::new(|ah: &mut ArithmeticHarness| {
                ah.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                Ok(())
            }),
        );

        ts.add_verb(
            "mul_two",
            FunctionVerb::new(|ah: &mut ArithmeticHarness| {
                let value = ah.value.load(std::sync::atomic::Ordering::SeqCst);
                ah.value
                    .store(value * 2, std::sync::atomic::Ordering::SeqCst);

                Ok(())
            }),
        );

        let tc = ts
            .parse_testcase(
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
            FunctionVerb::new(|ah: &mut ArithmeticHarness| {
                ah.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                Ok(())
            }),
        );

        ts.add_verb(
            "add",
            FunctionVerb::new(|ah: &mut ArithmeticHarness, num: usize| {
                ah.value.fetch_add(num, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }),
        );

        ts.add_verb(
            "mul_two",
            FunctionVerb::new(|ah: &mut ArithmeticHarness| {
                let value = ah.value.load(std::sync::atomic::Ordering::SeqCst);
                ah.value
                    .store(value * 2, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }),
        );

        let tc = ts
            .parse_testcase(
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

        let mut ah = ArithmeticHarness {
            value: AtomicUsize::new(0),
        };

        tc[0].run(&mut ah).unwrap();

        assert_eq!(ah.value.load(std::sync::atomic::Ordering::SeqCst), 60);
    }
}
