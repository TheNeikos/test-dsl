//! Test various error outputs

use test_dsl::condition::Condition;
use test_dsl::verb::FunctionVerb;

#[test]
fn check_invalid() {
    let ts = test_dsl::TestDsl::<()>::new();

    let tc = ts.parse_testcase(
        r#"
            testcase {
            }
            tetcase {
            }
            foobar {
            }
            asd
        "#,
    );

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));
}

#[test]
fn check_unknown() {
    let ts = test_dsl::TestDsl::<()>::new();

    let tc = ts.parse_testcase(
        r#"
            testcase {
                repeat 2 {
                    not_found
                }
            }
        "#,
    );

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));
}

#[test]
fn check_missing_argument() {
    let ts = test_dsl::TestDsl::<()>::new();

    let tc = ts.parse_testcase(
        r#"
            testcase {
                repeat {
                }
            }
        "#,
    );

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));
}

#[test]
fn check_argument_wrong_type() {
    let ts = test_dsl::TestDsl::<()>::new();

    let tc = ts.parse_testcase(
        r#"
            testcase {
                repeat hello {
                }
            }
        "#,
    );

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));
}

#[test]
fn check_argument_wrong_type_verb() {
    let mut ts = test_dsl::TestDsl::<()>::new();

    ts.add_verb(
        "foobar",
        FunctionVerb::new(|_: &mut (), _: usize| {
            // Nothing
            Ok(())
        }),
    );

    let tc = ts
        .parse_testcase(
            r#"
            testcase {
                foobar
            }
        "#,
        )
        .unwrap()[0]
        .run(&mut ());

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));

    let tc = ts
        .parse_testcase(
            r#"
            testcase {
                foobar not_a_number
            }
        "#,
        )
        .unwrap()[0]
        .run(&mut ());

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));
}

#[test]
fn check_verb_panic_fail() {
    let mut ts = test_dsl::TestDsl::<()>::new();

    ts.add_verb("foobar", FunctionVerb::new(|_: &mut (), _: usize| panic!()));

    let tc = ts
        .parse_testcase(
            r#"
            testcase {
                foobar 2 {
                    ofoo
                }
            }
        "#,
        )
        .unwrap()[0]
        .run(&mut ());

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));
}

#[test]
fn check_conditions() {
    let mut ts = test_dsl::TestDsl::<()>::new();

    ts.add_condition("is_true", Condition::new_now(|_h: &()| Ok(true)));
    ts.add_condition("is_false", Condition::new_now(|_h: &()| Ok(false)));

    let testcases = ts
        .parse_testcase(
            r#"
            testcase {
                assert {
                    is_true
                }
            }

            testcase {
                assert {
                    is_false
                }
            }
        "#,
        )
        .unwrap();

    // Check that its true
    testcases[0].run(&mut ()).unwrap();

    let is_false = testcases[1].run(&mut ());

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(is_false.unwrap_err())));
}

#[test]
fn check_arithmetic() {
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
}
