use insta::{assert_yaml_snapshot_matches, Settings};
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
    settings.run(|| {
        assert_yaml_snapshot_matches!(&map, @r###"
       ⋮---
       ⋮a: first value
       ⋮b: second value
       ⋮c: third value
       ⋮d: fourth value
        "###);
    });
}
