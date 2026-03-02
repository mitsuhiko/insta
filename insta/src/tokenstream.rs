//! `TokenStream` snapshot support helpers.
//!
//! This module provides utilities for comparing and formatting
//! [`proc_macro2::TokenStream`] values for snapshot testing.

use crate::Settings;
use proc_macro2::TokenStream;

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
/// the raw `TokenStream::to_string()` output if formatting fails.
///
/// The function attempts to parse the tokens as valid Rust code and format
/// them nicely. If parsing fails (e.g., for partial code fragments), it
/// returns the raw string representation.
pub fn pretty_print(tokens: &TokenStream) -> String {
    let format = Settings::with(|s| s.format_tokens());

    if !format {
        return tokens.to_string();
    }

    // Try direct parsing as a file (for complete items like structs, functions, etc.)
    // unparse always appends a trailing newline; trim it so single-item output
    // doesn't trigger the multiline path in pretty_print_for_inline
    if let Ok(mut file) = syn::parse2(tokens.clone()) {
        // Normalize doc attribute whitespace so that inline token snapshots
        // round-trip correctly. When `to_inline_tokens` indents snapshot
        // content in source files, `/** */` block doc comments absorb that
        // indentation as part of their string value (since `quote!` preserves
        // whitespace verbatim). Stripping common leading whitespace here
        // (similar to what rustdoc does) makes the comparison idempotent.
        normalize_doc_attrs(&mut file);
        return prettier_please::unparse(&file).trim_end().to_string();
    }

    // Try parsing as an expression (for code fragments)
    match syn::parse2::<syn::Expr>(tokens.clone()) {
        Ok(expr) => return prettier_please::unparse_expr(&expr).trim_end().to_string(),
        Err(err) => {
            crate::elog!(
                "warning: tokenstream snapshot could not be parsed as valid Rust; using raw formatting: {err}"
            );
        }
    }

    // Fallback: just use TokenStream::to_string()
    tokens.to_string()
}

/// Strip common leading whitespace from all `#[doc = "..."]` attribute values
/// in a `syn::File`. This mirrors the dedent behavior of rustdoc and ensures
/// that indentation added by inline snapshot formatting doesn't become part
/// of the doc content.
fn normalize_doc_attrs(file: &mut syn::File) {
    use syn::visit_mut::VisitMut;

    struct DocNormalizer;

    impl VisitMut for DocNormalizer {
        fn visit_attribute_mut(&mut self, attr: &mut syn::Attribute) {
            if !attr.path().is_ident("doc") {
                return;
            }
            if let syn::Meta::NameValue(nv) = &mut attr.meta {
                if let syn::Expr::Lit(lit) = &mut nv.value {
                    if let syn::Lit::Str(s) = &mut lit.lit {
                        let original = s.value();
                        let dedented = dedent_doc_content(&original);
                        if dedented != original {
                            *s = syn::LitStr::new(&dedented, s.span());
                        }
                    }
                }
            }
        }
    }

    DocNormalizer.visit_file_mut(file);
}

/// Dedent a doc comment string by removing common leading whitespace,
/// similar to rustdoc's behavior.
fn dedent_doc_content(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();

    // Find minimum indentation across non-empty lines (skip the first line
    // which is typically just "\n" after `/**`)
    let min_indent = lines
        .iter()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min();

    let Some(min_indent) = min_indent else {
        return s.to_string();
    };

    if min_indent == 0 {
        return s.to_string();
    }

    // Rebuild the string, stripping `min_indent` spaces from each line after the first
    let mut result = String::with_capacity(s.len());
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        if i == 0 {
            result.push_str(line);
        } else if line.trim().is_empty() {
            // Preserve empty lines as-is
        } else if line.len() >= min_indent {
            result.push_str(&line[min_indent..]);
        } else {
            result.push_str(line);
        }
    }

    // Preserve trailing newline if original had one
    if s.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }

    result
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

    #[test]
    fn test_dedent_doc_content_no_indent() {
        assert_eq!(dedent_doc_content("\nfoo\nbar"), "\nfoo\nbar");
    }

    #[test]
    fn test_dedent_doc_content_uniform_indent() {
        assert_eq!(dedent_doc_content("\n    foo\n    bar"), "\nfoo\nbar");
    }

    #[test]
    fn test_dedent_doc_content_mixed_indent() {
        assert_eq!(
            dedent_doc_content("\n        foo\n            bar\n        baz"),
            "\nfoo\n    bar\nbaz"
        );
    }

    #[test]
    fn test_dedent_doc_content_with_empty_lines() {
        assert_eq!(dedent_doc_content("\n    foo\n\n    bar"), "\nfoo\n\nbar");
    }

    #[test]
    fn test_dedent_doc_content_single_line() {
        assert_eq!(
            dedent_doc_content(" just a single line"),
            " just a single line"
        );
    }

    #[test]
    fn test_pretty_print_normalizes_indented_block_doc_comment() {
        // Simulate what happens when to_inline_tokens adds indentation to
        // /** */ content: quote! captures the whitespace as part of the doc string.
        let tokens_indented = quote! {
            /**
                Indented content
                more stuff*/
            struct Foo;
        };
        let tokens_no_indent = quote! {
                    /**
        Indented content
        more stuff*/
                    struct Foo;
                };
        // Both should produce identical output after normalization
        assert_eq!(
            pretty_print(&tokens_indented),
            pretty_print(&tokens_no_indent),
        );
    }

    #[test]
    fn test_pretty_print_inline_roundtrip_with_block_doc_comment() {
        // The actual value (from codegen, no extra indent in doc content)
        let actual = quote! {
                    /**
        Indented content
        more stuff*/
                    struct Foo;
                };
        // Simulate the reference value after to_inline_tokens added 12 spaces
        let reference = quote! {
            /**
            Indented content
            more stuff*/
            struct Foo;
        };
        assert_eq!(
            pretty_print_for_inline(&actual),
            pretty_print_for_inline(&reference),
        );
    }
}
