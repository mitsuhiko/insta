use insta::{allow_duplicates, assert_debug_snapshot};

#[test]
fn test_basic_duplicates_passes() {
    allow_duplicates! {
        for x in (0..10).step_by(2) {
            let is_even = x % 2 == 0;
            assert_debug_snapshot!(is_even, @"true");
        }
    }
}

#[test]
#[should_panic = "snapshot assertion for 'basic_duplicates_assertion_failed' failed in line"]
fn test_basic_duplicates_assertion_failed() {
    allow_duplicates! {
        for x in (0..10).step_by(3) {
            let is_even = x % 2 == 0;
            assert_debug_snapshot!(is_even, @"true");
        }
    }
}
