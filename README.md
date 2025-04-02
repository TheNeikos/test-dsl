# test-dsl is a test-helper library to write your own DSLs for testing

[![Crates.io Version](https://img.shields.io/crates/v/test-dsl)](https://crates.io/crates/test-dsl)
[![docs.rs (with version)](https://img.shields.io/docsrs/test-dsl/latest)](https://docs.rs/test-dsl)

```sh
cargo add --dev test-dsl
```

`test-dsl` allows you define a set of verbs and conditions, to more easily
concentrate on authoring tests.

Instead of copy-pasting boilerplate and creating hard-to-read tests, this crate
allows you to distill the behaviour of your library or application into small
actions called 'verbs'.

An example test for an imaginary "http client" crate could look like this:

```kdl
testcase {
    create_mock_server "example.com"
    create_client "sut"
    connect client="sut" server="example.com"

    assert {
        check_last_connection status=200
        verify_cache client="sut" url="example.com"
    }
}
```

## How to use it

Using `test-dsl` is straightforward:

- You define a test harness
- You define a set of 'verbs' that will allow you to act on your test harness
- You define a set of 'conditions' that you will be able to assert during your tests

For example, a fairly simple test-setup to check arithmetic can be defined as follows:

```rust
use std::sync::Arc;
use test_dsl::condition::Condition;
use test_dsl::verb::FunctionVerb;
use miette::NamedSource;

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
    .parse_testcase(
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
    )
    .unwrap();

// Check that its true
testcases[0].run(&mut 0).unwrap();
testcases[1].run(&mut 0).unwrap();
```

## Builtin verbs

The following verbs come builtin:

- `repeat <number> { .. }`: it allows for repetition of a given block. Used as such:
    ```kdl
    testcase {
        repeat 3 {
            print "Hello World"

            print "World Hello"
        }
    }
    ```

- `group { .. }`: it allows to group verbs together. Used as such:
    ```kdl
    testcase {
        group {
            print "Hello"
            print "World"
        }
    }
    ```
    NB: There is currently not much use to groups, but this may change in the future

- `assert { .. }`: it allows to assert a list of conditions. Used as such:
    ```kdl
    testcase {
        send_message
        assert {
            message_was_sent
        }
    }
    ```
## How the different types relate to eachother

- The main type is [`TestDsl`](crate::TestDsl) which serves as the coordinator.
  Ideally you should have a single function creating this object that you can
  reuse.
  Each [`TestDsl`](crate::TestDsl) is generic over your test **Harness**. Which
  is basically the 'coordinator' of your test. Think of it like an all-seeing
  part of your system, that can kick-start functionalities you'd want to test.
  It's usually best if your harness only interacts with the to-be-tested types
  through their public functions. But depending on how you organize your code
  it might also be able to access the inner workings.
- The work-horses 🐴 of this crate are the [`Verb`](crate::verb::Verb)
  implementations. You can implement it yourself, or you can use
  [`FunctionVerb`](crate::verb::FunctionVerb) for quick in-line verb
  definitions. [`FunctionVerb`](crate::verb::FunctionVerb) accepts closures
  which take your harness as well as arguments for your verb.
- Closely behind are the [`TestCondition`](crate::condition::TestCondition)s.
  They allow for verifying your invariants. Similarly to verbs, you can
  implement the trait yourself, or use the
  [`Condition`](crate::condition::Condition) helper.
- [`ParseArguments`](crate::argument::ParseArguments) is the bridge between
  `kdl` and `test_dsl`. It allows verbs and conditions to accept input in form
  of arguments and child nodes, and put it into a form that the
  verbs/conditions can then make use of.
- [`VerbInstance`](crate::VerbInstance) &
  [`ConditionInstance`](crate::ConditionInstance) are both fully-parsed and
  ready to run verbs & conditions. They are created from `TestDsl` instances.
  Mainly used in `ParseArguments` implementations.
