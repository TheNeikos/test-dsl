# test-dsl is a test-helper library to write your own DSLs

[![Crates.io Version](https://img.shields.io/crates/v/test-dsl)](https://crates.io/crates/test-dsl)
[![docs.rs (with version)](https://img.shields.io/docsrs/test-dsl/latest)](https://docs.rs/test-dsl)

```sh
cargo add --dev test-dsl
```

`test-dsl` allows you define a set of verbs and conditions, to more easily
concentrate on authoring tests.

## How to use

Using `test-dsl` is straightforward:

- You define a test harness
- You define a set of 'verbs' that will allow you to act on your test harness
- You define a set of 'conditions' that you will be able to assert during your tests

For example, a fairly simple test-setup to check arithmetic can be defined as follows:

```rust
let mut ts = test_dsl::TestDsl::<usize>::new();

ts.add_condition("is_fortytwo", Condition::new_now(|h: &usize| Ok(*h == 42)));
ts.add_condition(
    "is_equal",
    Condition::new_now(|h: &usize, num: usize| Ok(*h == num)),
);

ts.add_verb(
    "add",
    FunctionVerb::new(|h: &mut usize, num: usize| {
        *h += num;
        Ok(())
    }),
);

ts.add_verb(
    "mul",
    FunctionVerb::new(|h: &mut usize, num: usize| {
        *h *= num;
        Ok(())
    }),
);

let testcases = ts
    .parse_document(NamedSource::new(
        "test.kdl",
        Arc::from(
            r#"
        testcase {
            add 21
            mul 2
            assert {
                is_fortytwo
            }
        }

        testcase {
            add 10
            mul 10
            assert {
                is_equal 100
            }
        }
    "#,
        ),
    ))
    .unwrap();

// Check that its true
testcases[0].run(&mut 0).unwrap();
testcases[1].run(&mut 0).unwrap();
```
