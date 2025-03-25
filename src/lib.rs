use std::collections::HashMap;

pub trait TestVerb<H>: 'static {
    fn run(&self, harness: &H);
    fn clone_box(&self) -> Box<dyn TestVerb<H>>;
}

impl<H: 'static> Clone for Box<dyn TestVerb<H>> {
    fn clone(&self) -> Self {
        let this: &dyn TestVerb<H> = &**self;
        this.clone_box()
    }
}

impl<F, H: 'static> TestVerb<H> for F
where
    F: Fn(&H) + 'static,
    F: Clone,
{
    fn run(&self, harness: &H) {
        (self)(harness)
    }

    fn clone_box(&self) -> Box<dyn TestVerb<H>> {
        Box::new(self.clone())
    }
}

pub trait TestCondition<H>: 'static {}

pub struct TestDsl<H> {
    verbs: HashMap<String, Box<dyn TestVerb<H>>>,
    conditions: HashMap<String, Box<dyn TestCondition<H>>>,
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

    pub fn add_conditions(&mut self, name: impl AsRef<str>, condition: impl TestCondition<H>) {
        let existing = self
            .conditions
            .insert(name.as_ref().to_string(), Box::new(condition));

        assert!(existing.is_none());
    }

    pub fn parse_document(&self, input: &str) -> miette::Result<Vec<TestCase<H>>> {
        let document = kdl::KdlDocument::parse(input)?;

        let mut cases = vec![];

        for testcase_node in document.nodes() {
            if testcase_node.name().value() != "testcase" {
                return Err(miette::diagnostic!("expected a testcase").into());
            }

            let mut testcase = TestCase::new();

            for verb_node in testcase_node.iter_children() {
                testcase.creators.push(self.parse_node(verb_node));
            }

            cases.push(testcase);
        }

        Ok(cases)
    }

    fn parse_node(&self, verb_node: &kdl::KdlNode) -> Box<dyn TestVerbCreator<H>> {
        match verb_node.name().value() {
            "repeat" => {
                let times = verb_node.get(0).unwrap().as_integer().unwrap() as usize;

                let block = verb_node
                    .iter_children()
                    .map(|node| self.parse_node(node))
                    .collect();

                Box::new(Repeat { times, block })
            }
            name => {
                let verb = self.verbs.get(name).unwrap().clone();

                Box::new(Identity { verb })
            }
        }
    }
}

struct Repeat<H> {
    times: usize,
    block: Vec<Box<dyn TestVerbCreator<H>>>,
}

impl<H: 'static> TestVerbCreator<H> for Repeat<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = Box<dyn TestVerb<H>>> + '_> {
        Box::new(
            std::iter::repeat_with(|| self.block.iter().flat_map(|c| c.get_test_verbs()))
                .take(self.times)
                .flatten(),
        )
    }
}

struct Identity<H> {
    verb: Box<dyn TestVerb<H>>,
}

impl<H: 'static> TestVerbCreator<H> for Identity<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = Box<dyn TestVerb<H>>>> {
        Box::new(std::iter::once(self.verb.clone()))
    }
}

trait TestVerbCreator<H> {
    fn get_test_verbs(&self) -> Box<dyn Iterator<Item = Box<dyn TestVerb<H>>> + '_>;
}

pub struct TestCase<H> {
    creators: Vec<Box<dyn TestVerbCreator<H>>>,
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

    use crate::TestDsl;

    struct ArithmeticHarness {
        value: AtomicUsize,
    }

    #[test]
    fn simple_test() {
        let mut ts = TestDsl::<ArithmeticHarness>::new();
        ts.add_verb("add_one", |ah: &ArithmeticHarness| {
            ah.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });

        ts.add_verb("mul_two", |ah: &ArithmeticHarness| {
            let value = ah.value.load(std::sync::atomic::Ordering::SeqCst);
            ah.value
                .store(value * 2, std::sync::atomic::Ordering::SeqCst);
        });

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
        ts.add_verb("add_one", |ah: &ArithmeticHarness| {
            ah.value.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });

        ts.add_verb("mul_two", |ah: &ArithmeticHarness| {
            let value = ah.value.load(std::sync::atomic::Ordering::SeqCst);
            ah.value
                .store(value * 2, std::sync::atomic::Ordering::SeqCst);
        });

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
}
