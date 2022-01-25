#![cfg(feature = "redactions")]

use std::collections::{BTreeMap, HashMap, HashSet};

use insta::_macro_support::Selector;
use insta::{
    assert_debug_snapshot, assert_json_snapshot, assert_yaml_snapshot, sorted_redaction,
    with_settings, Settings,
};
use serde::Serialize;

use similar_asserts::assert_eq;

#[test]
fn test_selector_parser() {
    macro_rules! assert_selector_snapshot {
        ($short:expr, $sel:expr) => {
            assert_debug_snapshot!($short, Selector::parse($sel).unwrap());
        };
    }

    assert_selector_snapshot!("foo_bar", ".foo.bar");
    assert_selector_snapshot!("foo_bar_alt", ".foo[\"bar\"]");
    assert_selector_snapshot!("foo_bar_full_range", ".foo.bar[]");
    assert_selector_snapshot!("foo_bar_range_to", ".foo.bar[:10]");
    assert_selector_snapshot!("foo_bar_range_from", ".foo.bar[10:]");
    assert_selector_snapshot!("foo_bar_range", ".foo.bar[10:20]");
    assert_selector_snapshot!("foo_bar_deep", ".foo.bar.**");
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
    insta::assert_csv_snapshot!("user_csv", &User {
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
    insta::assert_ron_snapshot!("user_ron", &User {
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
    insta::assert_toml_snapshot!("user_toml", &User {
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

    let visitor = Role::Visitor {
        id: "my-id".into(),
        name: "my-name".into(),
    };
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

#[test]
fn test_struct_array_redaction() {
    #[derive(Serialize)]
    pub struct Product {
        _id: String,
        product_name: String,
    }

    #[derive(Serialize)]
    pub struct Checkout {
        _id: String,
        products: Vec<Product>,
    }

    let checkout = Checkout {
        _id: "checkout/1".to_string(),
        products: vec![
            Product {
                _id: "product/1".to_string(),
                product_name: "a car".to_string(),
            },
            Product {
                _id: "product/2".to_string(),
                product_name: "a boat".to_string(),
            },
        ],
    };

    insta::assert_yaml_snapshot!(vec![checkout], {
        "[]._id" => "[checkout_id]",
        "[].products[]._id" => "[product_id]",
        "[].products[].product_name" => "[product_name]",
    });
}

#[test]
fn test_map_key_redaction() {
    #[derive(Serialize, Hash, PartialEq, PartialOrd, Eq, Ord)]
    struct Key {
        bucket: u32,
        value: u32,
    }

    #[derive(Serialize)]
    struct Foo {
        hm: HashMap<Key, u32>,
        btm: BTreeMap<(u32, u32), u32>,
    }

    let mut hm = HashMap::new();
    hm.insert(
        Key {
            bucket: 1,
            value: 0,
        },
        42,
    );
    let mut btm = BTreeMap::new();
    btm.insert((0, 0), 23);
    let foo = Foo { hm, btm };

    insta::assert_yaml_snapshot!(foo, {
        ".hm.$key.bucket" => "[bucket]",
        ".btm.$key" => "[key]",
    });
}

#[test]
fn test_ordering() {
    #[derive(Debug, Serialize)]
    pub struct User {
        id: u64,
        username: String,
        flags: HashSet<String>,
    }

    let mut settings = Settings::new();
    settings.add_redaction(".id", "[id]");
    settings.sort_selector(".flags");
    settings.bind(|| {
        assert_json_snapshot!(
            "user_json_flags",
            &User {
                id: 122,
                username: "jason_doe".to_string(),
                flags: vec!["zzz".into(), "foo".into(), "aha".into(), "is_admin".into()]
                    .into_iter()
                    .collect(),
            }
        );
    });
}

#[test]
fn test_ordering_newtype_set() {
    #[derive(Debug, Serialize)]
    pub struct MySet(HashSet<String>);

    #[derive(Debug, Serialize)]
    pub struct User {
        id: u64,
        username: String,
        flags: MySet,
    }

    assert_json_snapshot!(
        "user_json_flags_alt",
        &User {
            id: 122,
            username: "jason_doe".to_string(),
            flags: MySet(vec!["zzz".into(), "foo".into(), "aha".into(), "is_admin".into()]
                .into_iter()
                .collect()),
        },
        {
            "." => sorted_redaction(),
            ".flags" => sorted_redaction()
        }
    );
}
