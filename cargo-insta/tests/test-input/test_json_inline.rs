use serde::Serialize;

#[derive(Serialize)]
struct User {
    id: u64,
    email: String,
}

#[test]
fn test_json_snapshot() {
    let user = User {
        id: 42,
        email: "john.doe@example.com".into(),
    };
    insta::assert_json_snapshot!(&user, {
        ".id" => "[user_id]",
    }, @"");
}

#[test]
fn test_json_snapshot_trailing_comma() {
    let user = User {
        id: 42,
        email: "john.doe@example.com".into(),
    };
    insta::assert_compact_json_snapshot!(
        &user,
        @"",
    );
}

#[test]
fn test_json_snapshot_trailing_comma_redaction() {
    let user = User {
        id: 42,
        email: "john.doe@example.com".into(),
    };
    insta::assert_json_snapshot!(
        &user,
        {
            ".id" => "[user_id]",
        },
        @"",
    );
}
