use insta::{
    assert_debug_snapshot, assert_display_snapshot, assert_json_snapshot, assert_yaml_snapshot,
};
use std::fmt;

#[test]
fn test_debug_vector() {
    assert_debug_snapshot!("debug_vector", vec![1, 2, 3]);
}

#[test]
fn test_unnamed_debug_vector() {
    assert_debug_snapshot!(vec![1, 2, 3]);
    assert_debug_snapshot!(vec![1, 2, 3, 4]);
    assert_debug_snapshot!(vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_yaml_vector() {
    assert_yaml_snapshot!("yaml_vector", vec![1, 2, 3]);
}

#[test]
fn test_unnamed_yaml_vector() {
    assert_yaml_snapshot!(vec![1, 2, 3]);
    assert_yaml_snapshot!(vec![1, 2, 3, 4]);
    assert_yaml_snapshot!(vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_json_vector() {
    assert_json_snapshot!("json_vector", vec![1, 2, 3]);
}

#[test]
fn test_unnamed_json_vector() {
    assert_json_snapshot!(vec![1, 2, 3]);
    assert_json_snapshot!(vec![1, 2, 3, 4]);
    assert_json_snapshot!(vec![1, 2, 3, 4, 5]);
}

mod nested {
    #[test]
    fn test_nested_module() {
        use insta::assert_snapshot;
        assert_snapshot!("aoeu");
    }
}

struct TestDisplay;

impl fmt::Display for TestDisplay {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "TestDisplay struct")
    }
}

#[test]
fn test_display() {
    let td = TestDisplay;
    assert_display_snapshot!("display", td);
}

#[test]
fn test_unnamed_display() {
    let td = TestDisplay;
    assert_display_snapshot!(td);
    assert_display_snapshot!("whatever");
}
