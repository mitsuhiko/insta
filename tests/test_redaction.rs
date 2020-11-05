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

#[cfg(feature = "csv")]
#[test]
fn test_with_random_value_csv() {
    use insta::assert_csv_snapshot;
    assert_csv_snapshot!("user_csv", &User {
        id: 44,
        username: "julius_csv".to_string(),
        email: Email("julius@example.com".to_string()),
        extra: "".to_string(),
    }, {
        ".id" => "[id]"
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

#[cfg(feature = "toml")]
#[test]
fn test_with_random_value_toml() {
    use insta::assert_toml_snapshot;
    assert_toml_snapshot!("user_toml", &User {
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
fn test_redact_newtype_struct() {
    #[derive(Serialize)]
    pub struct UserWrapper(User);

    let wrapper = UserWrapper(User {
        id: 42,
        username: "john_doe".to_string(),
        email: Email("john@example.com".to_string()),
        extra: "".to_string(),
    });

    assert_json_snapshot!(wrapper, {
        r#".id"# => "[id]"
    }, @r###"
    {
      "id": "[id]",
      "username": "john_doe",
      "email": "john@example.com",
      "extra": ""
    }
    "###);
}

#[test]
fn test_redact_newtype_enum() {
    #[derive(Serialize)]
    pub enum Role {
        Admin(User),
        Visitor { id: String, name: String },
    }

    let visitor = Role::Visitor { id: "my-id".into(), name: "my-name".into() };
    assert_yaml_snapshot!(visitor, {
        r#".id"# => "[id]",
    }, @r###"
    ---
    Visitor:
      id: "[id]"
      name: my-name
    "###);

    let admin = Role::Admin(User {
        id: 42,
        username: "john_doe".to_string(),
        email: Email("john@example.com".to_string()),
        extra: "".to_string(),
    });
    assert_yaml_snapshot!(admin, {
        r#".id"# => "[id]",
    }, @r###"
    ---
    Admin:
      id: "[id]"
      username: john_doe
      email: john@example.com
      extra: ""
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
