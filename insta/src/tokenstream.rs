//! `TokenStream` snapshot support helpers.
//!
//! This module provides utilities for comparing and formatting
//! [`proc_macro2::TokenStream`] values for snapshot testing.

use crate::Settings;
use proc_macro2::TokenStream;
use syn::visit_mut::VisitMut;

/// Pretty-print a `TokenStream` for use as an inline snapshot value.
///
/// This formats the tokens nicely and ensures the output follows insta's
/// inline snapshot conventions: multiline content starts with a leading
/// newline so it aligns properly in the source file.
pub fn pretty_print_for_inline(tokens: &TokenStream) -> String {
    let pretty = pretty_print(tokens);
    // Multiline inline snapshots must start with a newline
    if pretty.contains('\n') {
        format!("\n{}\n", pretty.trim_end())
    } else {
        pretty
    }
}

/// Pretty-print a `TokenStream` using `prettier-please`, falling back to
/// [`TokenStream::to_string()`] if formatting fails.
///
/// The function attempts to parse the tokens as valid Rust code and format
/// them nicely. If parsing fails (e.g., for partial code fragments), it
/// returns the raw string representation.
pub fn pretty_print(tokens: &TokenStream) -> String {
    let (format, ignore_docs) =
        Settings::with(|s| (s.format_tokens(), s.ignore_docs_for_tokens()));

    if !format {
        return tokens.to_string();
    }

    // Try direct parsing as a file (for complete items like structs, functions, etc.)
    if let Ok(file) = syn::parse2(tokens.clone()) {
        let file = remove_docs_if(file, ignore_docs);
        return prettier_please::unparse(&file);
    }

    // Try parsing as an expression (for code fragments)
    if let Ok(expr) = syn::parse2::<syn::Expr>(tokens.clone()) {
        let expr = remove_docs_if(expr, ignore_docs);
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
///
/// By default, doc attributes (`#[doc = "..."]`) are stripped before comparison.
/// This behavior can be disabled via [`Settings::set_ignore_docs_for_tokens`].
pub fn tokens_equal(a: &TokenStream, b: &TokenStream) -> bool {
    // Check if we should ignore docs (default: true)
    let ignore_docs = Settings::with(|s| s.ignore_docs_for_tokens());

    // First, try to parse both as files
    if let (Ok(a_file), Ok(b_file)) = (
        syn::parse2::<syn::File>(a.clone()),
        syn::parse2::<syn::File>(b.clone()),
    ) {
        let (a_file, b_file) = remove_docs_if((a_file, b_file), ignore_docs);
        return a_file == b_file;
    }

    // Next, try to parse both as expressions
    if let (Ok(a_expr), Ok(b_expr)) = (
        syn::parse2::<syn::Expr>(a.clone()),
        syn::parse2::<syn::Expr>(b.clone()),
    ) {
        let (a_expr, b_expr) = remove_docs_if((a_expr, b_expr), ignore_docs);
        return a_expr == b_expr;
    }

    // Fallback: compare the raw token streams directly
    a.to_string() == b.to_string()
}

/// Visitor that removes all `#[doc = "..."]` attributes.
struct DocRemover;

impl VisitMut for DocRemover {
    fn visit_attributes_mut(&mut self, attrs: &mut Vec<syn::Attribute>) {
        attrs.retain(|attr| !attr.path().is_ident("doc"));
    }
}

trait RemovableDocs {
    fn remove_docs(self) -> Self;
}

impl RemovableDocs for syn::Expr {
    fn remove_docs(mut self) -> Self {
        DocRemover.visit_expr_mut(&mut self);
        self
    }
}

impl RemovableDocs for syn::File {
    fn remove_docs(mut self) -> Self {
        DocRemover.visit_file_mut(&mut self);
        self
    }
}

impl<T: RemovableDocs> RemovableDocs for (T, T) {
    fn remove_docs(self) -> Self {
        (self.0.remove_docs(), self.1.remove_docs())
    }
}

fn remove_docs_if<T>(tokens: T, should_remove: bool) -> T
where
    T: RemovableDocs,
{
    if should_remove {
        tokens.remove_docs()
    } else {
        tokens
    }
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
        assert_snapshot!(pretty_print(&tokens), @r"
        struct MyStruct {
            field: i32,
        }
        ");
    }

    #[test]
    fn test_pretty_print_struct_with_docs_when_ignore_docs_enabled() {
        let tokens = quote! {
            /// This is a struct
            struct MyStruct {
                /// This is a field
                field: i32,
            }
        };
        crate::with_settings!({ignore_docs_for_tokens => true}, {
            assert_snapshot!(pretty_print(&tokens), @r"
            struct MyStruct {
                field: i32,
            }
            ");
        });
    }

    #[test]
    fn test_pretty_print_struct_with_docs_when_ignore_docs_disabled() {
        let tokens = quote! {
            /// This is a struct
            struct MyStruct {
                /// This is a field
                field: i32,
            }
        };
        crate::with_settings!({ignore_docs_for_tokens => false}, {
            assert_snapshot!(pretty_print(&tokens), @r"
            /// This is a struct
            struct MyStruct {
                /// This is a field
                field: i32,
            }
            ");
        });
    }

    #[test]
    fn test_pretty_print_expression() {
        let tokens = quote! { 1 + 2 };
        assert_snapshot!(pretty_print(&tokens), @"1 + 2");
    }

    #[test]
    fn test_pretty_print_empty() {
        let tokens = TokenStream::new();
        let pretty = pretty_print(&tokens);
        assert!(pretty.is_empty());
    }

    #[test]
    fn test_pretty_print_non_expression() {
        let tokens = quote! { Vec<u8> };
        assert_snapshot!(pretty_print(&tokens), @"Vec < u8 >");
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
    fn test_tokens_equal_whitespace_difference_non_expr() {
        let a = quote! { Vec < u8 > };
        let b = quote! { Vec<u8> };
        assert!(tokens_equal(&a, &b));
    }

    #[test]
    fn test_tokens_not_equal() {
        let a = quote! { struct Foo; };
        let b = quote! { struct Bar; };
        assert!(!tokens_equal(&a, &b));
    }

    #[test]
    fn test_tokens_equal_ignores_docs_when_enabled() {
        let with_docs = quote! {
            /// Documentation here
            struct Foo {
                /// Field doc
                x: i32,
            }
        };
        let without_docs = quote! {
            struct Foo {
                x: i32,
            }
        };
        crate::with_settings!({ignore_docs_for_tokens => true}, {
            assert!(tokens_equal(&with_docs, &without_docs));
        });
    }

    #[test]
    fn test_tokens_equal_respects_docs_when_disabled() {
        let with_docs = quote! {
            /// Documentation here
            struct Foo;
        };
        let without_docs = quote! {
            struct Foo;
        };
        crate::with_settings!({ignore_docs_for_tokens => false}, {
            assert!(!tokens_equal(&with_docs, &without_docs));
        });
    }

    #[test]
    fn test_pretty_print_raw_when_format_disabled() {
        let tokens = quote! {
            struct MyStruct {
                field: i32,
            }
        };
        crate::with_settings!({format_tokens => false}, {
            // Raw TokenStream::to_string() output — no prettier-please formatting
            assert_snapshot!(pretty_print(&tokens), @"struct MyStruct { field : i32 , }");
        });
    }

    #[test]
    fn test_pretty_print_expression_raw_when_format_disabled() {
        let tokens = quote! { 1 + 2 };
        crate::with_settings!({format_tokens => false}, {
            assert_snapshot!(pretty_print(&tokens), @"1 + 2");
        });
    }

    #[test]
    fn test_pretty_print_for_inline_raw_when_format_disabled() {
        let tokens = quote! {
            fn foo() {
                let x = 1;
            }
        };
        crate::with_settings!({format_tokens => false}, {
            // Single-line raw output, no newline wrapping
            assert_snapshot!(pretty_print_for_inline(&tokens), @"fn foo () { let x = 1 ; }");
        });
    }
}
