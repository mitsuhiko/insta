#[test]
fn test_embedded_test() {
    assert_snapshot!("embedded", "Just a string");
}

#[cfg(feature = "tokenstream")]
mod tokenstream_tests {
    use proc_macro2::TokenStream;
    use quote::quote;

    #[test]
    fn test_token_snapshot_file_based() {
        let tokens: TokenStream = quote! {
            struct MyStruct {
                field: i32,
            }
        };
        crate::assert_token_snapshot!("token_struct", tokens);
    }

    #[test]
    fn test_token_snapshot_inline_matching() {
        let tokens: TokenStream = quote! { struct Foo; };
        // This should pass because tokens are semantically equal
        crate::assert_token_snapshot!(tokens, @{ struct Foo; });
    }

    #[test]
    fn test_token_snapshot_inline_with_whitespace_difference() {
        // TokenStream comparison is semantic, so whitespace differences are ignored
        let tokens: TokenStream = quote! { struct   Foo   ; };
        crate::assert_token_snapshot!(tokens, @{ struct Foo; });
    }

    #[test]
    fn test_token_snapshot_function() {
        let tokens: TokenStream = quote! {
            fn hello_world() {
                println!("Hello, world!");
            }
        };
        crate::assert_token_snapshot!("token_function", tokens);
    }
}
