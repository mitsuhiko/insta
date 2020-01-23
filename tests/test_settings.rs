use insta::{assert_yaml_snapshot, with_settings, Settings};
use std::collections::HashMap;

#[test]
fn test_simple() {
    let mut map = HashMap::new();
    map.insert("a", "first value");
    map.insert("b", "second value");
    map.insert("c", "third value");
    map.insert("d", "fourth value");

    let mut settings = Settings::new();
    settings.set_sort_maps(true);
    settings.bind(|| {
        assert_yaml_snapshot!(&map, @r###"
        ---
        a: first value
        b: second value
        c: third value
        d: fourth value
        "###);
    });
}

#[test]
fn test_bound_to_thread() {
    let mut map = HashMap::new();
    map.insert("a", "first value");
    map.insert("b", "second value");
    map.insert("c", "third value");
    map.insert("d", "fourth value");

    let mut settings = Settings::new();
    settings.set_sort_maps(true);
    settings.bind_to_thread();
    assert_yaml_snapshot!(&map, @r###"
    ---
    a: first value
    b: second value
    c: third value
    d: fourth value
    "###);
}

#[test]
fn test_settings_macro() {
    let mut map = HashMap::new();
    map.insert("a", "first value");
    map.insert("b", "second value");
    map.insert("c", "third value");
    map.insert("d", "fourth value");

    with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(&map, @r###"
        ---
        a: first value
        b: second value
        c: third value
        d: fourth value
        "###);
    });
}

#[test]
fn test_snapshot_path() {
    with_settings!({snapshot_path => "snapshots2"}, {
        assert_yaml_snapshot!(vec![1, 2, 3]);
    });
}
