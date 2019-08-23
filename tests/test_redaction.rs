#![cfg(feature = "redactions")]

use insta::_macro_support::Selector;
use insta::{
    assert_debug_snapshot_matches, assert_json_snapshot_matches, assert_yaml_snapshot_matches,
    Settings,
};
use serde::Serialize;
use uuid::Uuid;

#[test]
fn test_selector_parser() {
    macro_rules! assert_selector {
        ($short:expr, $sel:expr) => {
            assert_debug_snapshot_matches!($short, Selector::parse($sel).unwrap());
        };
    }

    assert_selector!("foo_bar", ".foo.bar");
    assert_selector!("foo_bar_alt", ".foo[\"bar\"]");
    assert_selector!("foo_bar_full_range", ".foo.bar[]");
    assert_selector!("foo_bar_range_to", ".foo.bar[:10]");
    assert_selector!("foo_bar_range_from", ".foo.bar[10:]");
    assert_selector!("foo_bar_range", ".foo.bar[10:20]");
}

#[derive(Serialize)]
pub struct Email(String);

#[derive(Serialize)]
pub struct User {
    id: Uuid,
    username: String,
    email: Email,
    extra: String,
}

#[test]
fn test_with_random_value() {
    assert_yaml_snapshot_matches!("user", &User {
        id: Uuid::new_v4(),
        username: "john_doe".to_string(),
        email: Email("john@example.com".to_string()),
        extra: "".to_string(),
    }, {
        ".id" => "[uuid]"
    });
}

#[cfg(feature = "ron")]
#[test]
fn test_with_random_value_ron() {
    use insta::assert_ron_snapshot_matches;
    assert_ron_snapshot_matches!("user_ron", &User {
        id: Uuid::new_v4(),
        username: "john_ron".to_string(),
        email: Email("john@example.com".to_string()),
        extra: "".to_string(),
    }, {
        ".id" => "[uuid]"
    });
}

#[test]
fn test_with_random_value_json() {
    assert_json_snapshot_matches!("user_json", &User {
        id: Uuid::new_v4(),
        username: "jason_doe".to_string(),
        email: Email("jason@example.com".to_string()),
        extra: "ssn goes here".to_string(),
    }, {
        ".id" => "[uuid]",
        ".extra" => "[extra]"
    });
}

#[test]
fn test_with_random_value_json_settings() {
    let mut settings = Settings::new();
    settings.add_redaction(".id", "[uuid]");
    settings.add_redaction(".extra", "[extra]");
    settings.bind(|| {
        assert_json_snapshot_matches!(
            "user_json_settings",
            &User {
                id: Uuid::new_v4(),
                username: "jason_doe".to_string(),
                email: Email("jason@example.com".to_string()),
                extra: "ssn goes here".to_string(),
            }
        );
    });
}
