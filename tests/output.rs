#[test]
fn check_invalid() {
    let ts = test_dsl::TestDsl::<()>::new();

    let tc = ts.parse_document(
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

    let tc = ts.parse_document(
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

    let tc = ts.parse_document(
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

    let tc = ts.parse_document(
        r#"
            testcase {
                repeat hello {
                }
            }
        "#,
    );

    insta::assert_snapshot!(format!("{:?}", miette::Error::new(tc.unwrap_err())));
}
