use std::sync::Arc;

use miette::NamedSource;
use test_dsl::condition::Condition;
use test_dsl::verb::FunctionVerb;

#[test]
fn check_invalid() {
    let ts = test_dsl::TestDsl::<()>::new();

    let tc = ts.parse_document(NamedSource::new(
        "test.kdl",
        Arc::from(
            r#"
            testcase {
            }
            tetcase {
            }
            foobar {
            }
            asd
        "#,
        ),
    ));

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));
}

#[test]
fn check_unknown() {
    let ts = test_dsl::TestDsl::<()>::new();

    let tc = ts.parse_document(NamedSource::new(
        "test.kdl",
        Arc::from(
            r#"
            testcase {
                repeat 2 {
                    not_found
                }
            }
        "#,
        ),
    ));

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));
}

#[test]
fn check_missing_argument() {
    let ts = test_dsl::TestDsl::<()>::new();

    let tc = ts.parse_document(NamedSource::new(
        "test.kdl",
        Arc::from(
            r#"
            testcase {
                repeat {
                }
            }
        "#,
        ),
    ));

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));
}

#[test]
fn check_argument_wrong_type() {
    let ts = test_dsl::TestDsl::<()>::new();

    let tc = ts.parse_document(NamedSource::new(
        "test.kdl",
        Arc::from(
            r#"
            testcase {
                repeat hello {
                }
            }
        "#,
        ),
    ));

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
        .parse_document(NamedSource::new(
            "test.kdl",
            Arc::from(
                r#"
            testcase {
                foobar
            }
        "#,
            ),
        ))
        .unwrap()[0]
        .run(&mut ());

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));

    let tc = ts
        .parse_document(NamedSource::new(
            "test.kdl",
            Arc::from(
                r#"
            testcase {
                foobar not_a_number
            }
        "#,
            ),
        ))
        .unwrap()[0]
        .run(&mut ());

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));
}

#[test]
fn check_verb_panic_fail() {
    let mut ts = test_dsl::TestDsl::<()>::new();

    ts.add_verb("foobar", FunctionVerb::new(|_: &mut (), _: usize| panic!()));

    let tc = ts
        .parse_document(NamedSource::new(
            "test.kdl",
            Arc::from(
                r#"
            testcase {
                foobar 2 {
                    ofoo
                }
            }
        "#,
            ),
        ))
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
        .parse_document(NamedSource::new(
            "test.kdl",
            Arc::from(
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
            ),
        ))
        .unwrap();

    // Check that its true
    testcases[0].run(&mut ()).unwrap();

    let is_false = testcases[1].run(&mut ());

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(is_false.unwrap_err())));
}
