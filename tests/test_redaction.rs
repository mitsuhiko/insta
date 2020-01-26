#![cfg(feature = "redactions")]

use insta::_macro_support::Selector;
use insta::{
    assert_debug_snapshot, assert_json_snapshot, assert_yaml_snapshot, with_settings, Settings,
};
use serde::Serialize;

#[test]
fn test_selector_parser() {
    macro_rules! assert_selector {
        ($short:expr, $sel:expr) => {
            assert_debug_snapshot!($short, Selector::parse($sel).unwrap());
        };
    }

    assert_selector!("foo_bar", ".foo.bar");
    assert_selector!("foo_bar_alt", ".foo[\"bar\"]");
    assert_selector!("foo_bar_full_range", ".foo.bar[]");
    assert_selector!("foo_bar_range_to", ".foo.bar[:10]");
    assert_selector!("foo_bar_range_from", ".foo.bar[10:]");
    assert_selector!("foo_bar_range", ".foo.bar[10:20]");
    assert_selector!("foo_bar_deep", ".foo.bar.**");
}

#[derive(Serialize)]
pub struct Email(String);

#[derive(Serialize)]
pub struct User {
    id: u32,
    username: String,
    email: Email,
    extra: String,
}

#[test]
fn test_with_random_value() {
    assert_yaml_snapshot!("user", &User {
        id: 42,
        username: "john_doe".to_string(),
        email: Email("john@example.com".to_string()),
        extra: "".to_string(),
    }, {
        ".id" => "[id]"
    });
}

#[test]
fn test_with_random_value_inline_callback() {
    assert_yaml_snapshot!("user", &User {
        id: 23,
        username: "john_doe".to_string(),
        email: Email("john@example.com".to_string()),
        extra: "".to_string(),
    }, {
        ".id" => insta::dynamic_redaction(|value, path| {
            assert_eq!(path.to_string(), ".id");
            assert_eq!(value.as_u64().unwrap(), 23);
            "[id]"
        }),
    });
}

#[test]
fn test_with_random_value_and_trailing_comma() {
    assert_yaml_snapshot!("user", &User {
        id: 11,
        username: "john_doe".to_string(),
        email: Email("john@example.com".to_string()),
        extra: "".to_string(),
    }, {
        ".id" => "[id]",
    });
}

#[cfg(feature = "ron")]
#[test]
fn test_with_random_value_ron() {
    use insta::assert_ron_snapshot;
    assert_ron_snapshot!("user_ron", &User {
        id: 53,
        username: "john_ron".to_string(),
        email: Email("john@example.com".to_string()),
        extra: "".to_string(),
    }, {
        ".id" => "[id]"
    });
}

#[test]
fn test_with_random_value_json() {
    assert_json_snapshot!("user_json", &User {
        id: 9999,
        username: "jason_doe".to_string(),
        email: Email("jason@example.com".to_string()),
        extra: "ssn goes here".to_string(),
    }, {
        ".id" => "[id]",
        ".extra" => "[extra]"
    });
}

#[test]
fn test_with_random_value_json_settings() {
    let mut settings = Settings::new();
    settings.add_redaction(".id", "[id]");
    settings.add_redaction(".extra", "[extra]");
    settings.bind(|| {
        assert_json_snapshot!(
            "user_json_settings",
            &User {
                id: 122,
                username: "jason_doe".to_string(),
                email: Email("jason@example.com".to_string()),
                extra: "ssn goes here".to_string(),
            }
        );
    });
}

#[test]
fn test_with_callbacks() {
    let mut settings = Settings::new();
    settings.add_dynamic_redaction(".id", |value, path| {
        assert_eq!(path.to_string(), ".id");
        assert_eq!(value.as_u64().unwrap(), 1234);
        "[id]"
    });
    settings.bind(|| {
        assert_json_snapshot!(
            "user_json_settings_callback",
            &User {
                id: 1234,
                username: "jason_doe".to_string(),
                email: Email("jason@example.com".to_string()),
                extra: "extra here".to_string(),
            }
        );
    });
}

#[test]
fn test_with_random_value_json_settings2() {
    with_settings!({redactions => vec![
        (".id", "[id]".into()),
        (".extra", "[extra]".into()),
    ]}, {
        assert_json_snapshot!(
            &User {
                id: 975,
                username: "jason_doe".to_string(),
                email: Email("jason@example.com".to_string()),
                extra: "ssn goes here".to_string(),
            }
        );
    });
}

#[test]
fn test_redact_newtype() {
    #[derive(Serialize, Clone)]
    pub struct User {
        id: String,
        name: String,
    }

    #[derive(Serialize)]
    pub struct UserWrapper(User);

    let user = User {
        id: "my-id".into(),
        name: "my-name".into(),
    };
    let wrapper = UserWrapper(user.clone());

    // This works as expected
    assert_json_snapshot!(user, {
        r#".id"# => "[id]"
    }, @r###"
    {
      "id": "[id]",
      "name": "my-name"
    }
    "###);

    // This fails - 'id' is not redacted
    assert_json_snapshot!(wrapper, {
        r#".id"# => "[id]"
    }, @r###"
    {
      "id": "[id]",
      "name": "my-name"
    }
    "###);
}

#[test]
fn test_redact_recursive() {
    #[derive(Serialize)]
    pub struct Node {
        id: u64,
        next: Option<Box<Node>>,
    }

    let root = Node {
        id: 0,
        next: Some(Box::new(Node { id: 1, next: None })),
    };

    assert_json_snapshot!(root, {
        ".**.id" => "[id]",
    }, @r###"
    {
      "id": "[id]",
      "next": {
        "id": "[id]",
        "next": null
      }
    }
    "###);
}
