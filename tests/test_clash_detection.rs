use std::env;
use std::thread;

#[test]
fn test_clash_detection() {
    let value = env::var("INSTA_UPDATE");
    env::set_var("INSTA_UPDATE", "no");

    let err1 = thread::Builder::new()
        .name("test_foo_always_missing".into())
        .spawn(|| {
            insta::assert_debug_snapshot!(42);
        })
        .unwrap()
        .join()
        .unwrap_err();
    let err2 = thread::Builder::new()
        .name("foo_always_missing".into())
        .spawn(|| {
            insta::assert_debug_snapshot!(42);
        })
        .unwrap()
        .join()
        .unwrap_err();

    if let Ok(value) = value {
        env::set_var("INSTA_UPDATE", value);
    } else {
        env::remove_var("INSTA_UPDATE");
    }

    let s1 = err1.downcast_ref::<String>().unwrap();
    let s2 = err2.downcast_ref::<String>().unwrap();
    let mut values = vec![s1.as_str(), s2.as_str()];
    values.sort();
    assert_eq!(&values[..], vec![
        "Insta snapshot name clash detected between \'foo_always_missing\' and \'test_foo_always_missing\' in \'test_clash_detection\'. Rename one function.",
        "snapshot assertion for \'foo_always_missing\' failed in line 12",
    ]);
}
