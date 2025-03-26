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
