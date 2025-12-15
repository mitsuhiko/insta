//! `TokenStream` snapshot support helpers.
//!
//! This module provides utilities for comparing and formatting
//! [`proc_macro2::TokenStream`] values for snapshot testing.

use proc_macro2::TokenStream;

/// Pretty-print a `TokenStream` using `prettier-please`, falling back to
/// [`TokenStream::to_string()`] if formatting fails.
///
/// The function attempts to parse the tokens as valid Rust code and format
/// them nicely. If parsing fails (e.g., for partial code fragments), it
/// returns the raw string representation.
pub fn pretty_print(tokens: &TokenStream) -> String {
    // Try direct parsing as a file (for complete items like structs, functions, etc.)
    if let Ok(file) = syn::parse2(tokens.clone()) {
        return prettier_please::unparse(&file);
    }

    // Try parsing as an expression (for code fragments)
    if let Ok(expr) = syn::parse2::<syn::Expr>(tokens.clone()) {
        return prettier_please::unparse_expr(&expr);
    }

    // Fallback: just use TokenStream::to_string()
    tokens.to_string()
}

/// Compare two `TokenStream`s semantically.
///
/// `TokenStream`s are considered equal if they produce equivalent token sequences
/// after normalization (parsing and re-printing). This means whitespace and
/// formatting differences are ignored.
pub fn tokens_equal(a: &TokenStream, b: &TokenStream) -> bool {
    // First, try to parse both as files
    if let (Ok(a_file), Ok(b_file)) = (
        syn::parse2::<syn::File>(a.clone()),
        syn::parse2::<syn::File>(b.clone()),
    ) {
        return a_file == b_file;
    }

    // Next, try to parse both as expressions
    if let (Ok(a_expr), Ok(b_expr)) = (
        syn::parse2::<syn::Expr>(a.clone()),
        syn::parse2::<syn::Expr>(b.clone()),
    ) {
        return a_expr == b_expr;
    }

    // Fallback: compare the raw token streams directly
    a.to_string() == b.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn test_pretty_print_struct() {
        let tokens = quote! {
            struct MyStruct {
                field: i32,
            }
        };
        let pretty = pretty_print(&tokens);
        assert!(pretty.contains("struct MyStruct"));
        assert!(pretty.contains("field: i32"));
    }

    #[test]
    fn test_pretty_print_expression() {
        let tokens = quote! { 1 + 2 };
        let pretty = pretty_print(&tokens);
        assert!(pretty.contains("1") && pretty.contains("2"));
    }

    #[test]
    fn test_pretty_print_empty() {
        let tokens = TokenStream::new();
        let pretty = pretty_print(&tokens);
        assert!(pretty.is_empty());
    }

    #[test]
    fn test_tokens_equal_identical() {
        let a = quote! { struct Foo; };
        let b = quote! { struct Foo; };
        assert!(tokens_equal(&a, &b));
    }

    #[test]
    fn test_tokens_equal_whitespace_difference() {
        let a = quote! { struct Foo { x: i32 } };
        let b = quote! { struct Foo{x:i32} };
        // After normalization via TokenStream, these should be equal
        // Note: quote! already normalizes, so this tests the round-trip
        assert!(tokens_equal(&a, &b));
    }

    #[test]
    fn test_tokens_not_equal() {
        let a = quote! { struct Foo; };
        let b = quote! { struct Bar; };
        assert!(!tokens_equal(&a, &b));
    }
}
