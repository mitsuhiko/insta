#![cfg(feature = "filters")]

use insta::{assert_snapshot, with_settings};

#[test]
fn test_basic_filter() {
    with_settings!({filters => vec![
        (r"\b[[:xdigit:]]{8}\b", "[SHORT_HEX]")
    ]}, {
        assert_snapshot!("Hello DEADBEEF!", @"Hello [SHORT_HEX]!");
    })
}
