use std::env;
use std::thread;

fn test_foo_always_missing() {
    insta::assert_debug_snapshot!(42);
}

fn foo_always_missing() {
    insta::assert_debug_snapshot!(42);
}

#[test]
fn test_clash_detection() {
    let old_update_value = env::var("INSTA_UPDATE");
    let old_force_pass_value = env::var("INSTA_FORCE_PASS");
    env::set_var("INSTA_UPDATE", "no");
    env::set_var("INSTA_FORCE_PASS", "0");

    let err1 = thread::Builder::new()
        .spawn(|| {
            test_foo_always_missing();
        })
        .unwrap()
        .join()
        .unwrap_err();
    let err2 = thread::Builder::new()
        .spawn(|| {
            foo_always_missing();
        })
        .unwrap()
        .join()
        .unwrap_err();

    if let Ok(value) = old_update_value {
        env::set_var("INSTA_UPDATE", value);
    } else {
        env::remove_var("INSTA_UPDATE");
    }
    if let Ok(value) = old_force_pass_value {
        env::set_var("INSTA_FORCE_PASS", value);
    } else {
        env::remove_var("INSTA_FORCE_PASS");
    }

    let s1 = err1.downcast_ref::<String>().unwrap();
    let s2 = err2.downcast_ref::<String>().unwrap();
    let mut values = [s1.as_str(), s2.as_str()];
    values.sort();
    assert_eq!(&values[..], &vec![
        "Insta snapshot name clash detected between \'foo_always_missing\' and \'test_foo_always_missing\' in \'test_clash_detection\'. Rename one function.",
        "snapshot assertion for \'foo_always_missing\' failed in line 5",
    ][..]);
}
