use uuid::Uuid;
use insta::{Selector, Value};
use insta::{assert_debug_snapshot_matches, assert_serialized_snapshot_matches};
use serde::Serialize;

#[test]
fn test_redaction_basics() {
    let value: Value = serde_yaml::from_str(r#"{"x":{"y":42}}"#).unwrap();
    let selector = Selector::parse(".x.y").unwrap();
    let new_value = selector.redact(value, &Value::from("[redacted]"));

    assert_debug_snapshot_matches!("redaction_basics", &new_value);
}

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

#[test]
fn test_with_random_value() {
    #[derive(Serialize)]
    pub struct User {
        id: Uuid,
        username: String,
    }

    assert_serialized_snapshot_matches!("user", &User {
        id: Uuid::new_v4(),
        username: "john_doe".to_string(),
    }, {
        ".id" => "[uuid]"
    });
}
