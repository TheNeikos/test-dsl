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

    pub fn parse_document(
        &self,
        input: miette::NamedSource<Arc<str>>,
    ) -> Result<Vec<test_case::TestCase<H>>, error::TestParseError> {
        let document = kdl::KdlDocument::parse(input.inner())?;

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
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    use miette::NamedSource;

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
