#[cfg(feature = "png")]
mod test_png {
    static XEYES_PNG: &'static [u8] = include_bytes!("xeyes.png");
    static XEYES_ANGEL_PNG: &'static [u8] = include_bytes!("xeyes-angel.png");

    #[test]
    fn test_nameless() {
        insta::assert_png_snapshot!(XEYES_PNG.to_vec());
    }

    #[test]
    fn test_named() {
        insta::assert_png_snapshot!("named", XEYES_PNG.to_vec());
    }

    #[test]
    #[should_panic(expected = "snapshot assertion for 'named' failed")]
    fn test_named_failure() {
        // This image is different from the one used in test_named, so this test
        // should fail.
        insta::assert_png_snapshot!("named", XEYES_ANGEL_PNG.to_vec());
    }
}
