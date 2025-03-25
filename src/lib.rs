use std::collections::HashMap;

pub trait TestVerb<H>: 'static {}
pub trait TestCondition<H>: 'static {}

pub struct TestDsl<H> {
    verbs: HashMap<String, Box<dyn TestVerb<H>>>,
    conditions: HashMap<String, Box<dyn TestCondition<H>>>,
}

impl<H> Default for TestDsl<H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<H> TestDsl<H> {
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
}
