#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

use std::collections::HashMap;
use std::sync::Arc;

use condition::TestCondition;
use verb::TestVerb;

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
    verbs: HashMap<String, Box<dyn TestVerb<H>>>,
    conditions: HashMap<String, Box<dyn condition::TestCondition<H>>>,
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
        TestDsl {
            verbs: HashMap::default(),
            conditions: HashMap::default(),
        }
    }

    /// Add a single verb
    ///
    /// The name is used as-is in your testcases, the arguments are up to each individual [`TestVerb`] implementation.
    ///
    /// See [`FunctionVerb`](verb::FunctionVerb) for an easy to use way of defining verbs.
    pub fn add_verb(&mut self, name: impl AsRef<str>, verb: impl TestVerb<H>) {
        let existing = self.verbs.insert(name.as_ref().to_string(), Box::new(verb));
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
            .insert(name.as_ref().to_string(), Box::new(condition));

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

            for verb_node in testcase_node.iter_children() {
                match self.parse_verb(verb_node) {
                    Ok(verb) => testcase.creators.push(verb),
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

    fn parse_condition(
        &self,
        condition_node: &kdl::KdlNode,
    ) -> Result<Box<dyn TestConditionCreator<H>>, error::TestErrorCase> {
        self.conditions
            .get(condition_node.name().value())
            .ok_or_else(|| error::TestErrorCase::UnknownCondition {
                condition: condition_node.name().span(),
            })
            .map(|cond| {
                Box::new(DirectCondition {
                    condition: cond.clone(),
                    node: condition_node.clone(),
                }) as Box<_>
            })
    }

    fn parse_verb(
        &self,
        verb_node: &kdl::KdlNode,
    ) -> Result<Box<dyn TestVerbCreator<H>>, error::TestErrorCase> {
        match verb_node.name().value() {
            "repeat" => {
                let times = verb_node
                    .get(0)
                    .ok_or_else(|| error::TestErrorCase::MissingArgument {
                        parent: verb_node.name().span(),
                        missing: String::from("`repeat` takes one argument, the repetition count"),
                    })?
                    .as_integer()
                    .ok_or_else(|| error::TestErrorCase::WrongArgumentType {
                        parent: verb_node.name().span(),
                        argument: verb_node.iter().next().unwrap().span(),
                        expected: String::from("Expected an integer"),
                    })? as usize;

                let block = verb_node
                    .iter_children()
                    .map(|node| self.parse_verb(node))
                    .collect::<Result<_, _>>()?;

                Ok(Box::new(Repeat { times, block }))
            }
            "group" => Ok(Box::new(Group {
                block: verb_node
                    .iter_children()
                    .map(|n| self.parse_verb(n))
                    .collect::<Result<_, _>>()?,
            })),
            "assert" => Ok(Box::new(AssertConditions {
                conditions: verb_node
                    .iter_children()
                    .map(|node| self.parse_condition(node))
                    .collect::<Result<_, _>>()?,
            })),
            name => {
                let verb = self
                    .verbs
                    .get(name)
                    .ok_or_else(|| error::TestErrorCase::UnknownVerb {
                        verb: verb_node.name().span(),
                    })?
                    .clone();

                Ok(Box::new(DirectVerb {
                    verb,
                    node: verb_node.clone(),
                }))
            }
        }
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

trait TestVerbCreator<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = TestVerbInstance<'_, H>> + '_>;
}

trait TestConditionCreator<H> {
    fn get_test_conditions(&self) -> Box<dyn Iterator<Item = TestConditionInstance<'_, H>> + '_>;
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

struct DirectVerb<H> {
    verb: Box<dyn TestVerb<H>>,
    node: kdl::KdlNode,
}

impl<H: 'static> TestVerbCreator<H> for DirectVerb<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = TestVerbInstance<'_, H>> + '_> {
        Box::new(std::iter::once(TestVerbInstance {
            verb: self.verb.clone(),
            node: &self.node,
        }))
    }
}

struct DirectCondition<H> {
    condition: Box<dyn TestCondition<H>>,
    node: kdl::KdlNode,
}

impl<H: 'static> TestConditionCreator<H> for DirectCondition<H> {
    fn get_test_conditions(&self) -> Box<dyn Iterator<Item = TestConditionInstance<'_, H>> + '_> {
        Box::new(std::iter::once(TestConditionInstance {
            condition: self.condition.clone(),
            node: &self.node,
        }))
    }
}

struct AssertConditions<H> {
    conditions: Vec<Box<dyn TestConditionCreator<H>>>,
}

impl<H: 'static> TestVerbCreator<H> for AssertConditions<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = TestVerbInstance<'_, H>> + '_> {
        Box::new(
            self.conditions
                .iter()
                .flat_map(|cond| cond.get_test_conditions())
                .map(|cond| TestVerbInstance {
                    node: cond.node,
                    verb: Box::new(AssertVerb {
                        condition: cond.condition,
                    }),
                }),
        )
    }
}

struct AssertVerb<H> {
    condition: Box<dyn TestCondition<H>>,
}

impl<H: 'static> TestVerb<H> for AssertVerb<H> {
    fn run(&self, harness: &mut H, node: &kdl::KdlNode) -> Result<(), error::TestErrorCase> {
        self.condition.check_now(harness, node).and_then(|res| {
            res.then_some(())
                .ok_or_else(|| error::TestErrorCase::Error {
                    error: miette::miette!("Assert failed"),
                    label: node.span(),
                })
        })
    }

    fn clone_box(&self) -> Box<dyn TestVerb<H>> {
        Box::new(self.clone())
    }
}

impl<H: 'static> Clone for AssertVerb<H> {
    fn clone(&self) -> Self {
        AssertVerb {
            condition: self.condition.clone(),
        }
    }
}

struct TestConditionInstance<'p, H> {
    condition: Box<dyn TestCondition<H>>,
    node: &'p kdl::KdlNode,
}

struct TestVerbInstance<'p, H> {
    verb: Box<dyn TestVerb<H>>,
    node: &'p kdl::KdlNode,
}

impl<'p, H: 'static> TestVerbInstance<'p, H> {
    fn run<'h>(&'p self, harness: &'h mut H) -> Result<(), error::TestErrorCase> {
        self.verb.run(harness, self.node)
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
