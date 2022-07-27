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
#[allow(deprecated)]
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

    // put defaults back
    let settings = Settings::new();
    settings.bind_to_thread();
}

#[test]
fn test_bound_to_scope() {
    let mut map = HashMap::new();
    map.insert("a", "first value");
    map.insert("b", "second value");
    map.insert("c", "third value");
    map.insert("d", "fourth value");

    {
        let mut settings = Settings::new();
        settings.set_sort_maps(true);
        let _guard = settings.bind_to_scope();
        assert_yaml_snapshot!(&map, @r###"
        ---
        a: first value
        b: second value
        c: third value
        d: fourth value
        "###);
    }

    assert!(!Settings::clone_current().sort_maps());
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

#[test]
fn test_snapshot_no_module_prepending() {
    with_settings!({prepend_module_to_snapshot => false}, {
        assert_yaml_snapshot!(vec![1, 2, 3]);
    });
}

#[test]
fn test_snapshot_with_description() {
    with_settings!({description => "The snapshot are three integers"}, {
        assert_yaml_snapshot!(vec![1, 2, 3])
    });
}

#[test]
fn test_snapshot_with_description_and_info() {
    #[derive(serde::Serialize)]
    pub struct Info {
        env: std::collections::HashMap<&'static str, &'static str>,
        cmdline: Vec<&'static str>,
    }
    let info = Info {
        env: From::from([("ENVIRONMENT", "production")]),
        cmdline: vec!["my-tool", "run"],
    };
    with_settings!({description => "The snapshot are four integers", info => &info}, {
        assert_yaml_snapshot!(vec![1, 2, 3, 4])
    });
}

#[test]
fn test_with_settings_inherit() {
    with_settings!({sort_maps => true}, {
        with_settings!({description => "aha"}, {
            let settings = Settings::clone_current();
            assert!(settings.sort_maps());
            assert_eq!(settings.description(), Some("aha"));
        });
    });
}
