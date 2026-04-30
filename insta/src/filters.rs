use std::borrow::Cow;
use std::iter::FromIterator;
use std::iter::IntoIterator;

use regex::Regex;

/// Represents stored filters.
#[derive(Debug, Default, Clone)]
#[cfg_attr(docsrs, doc(cfg(feature = "filters")))]
pub struct Filters {
    rules: Vec<(Regex, String)>,
}

impl<'a, I> From<I> for Filters
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
{
    fn from(value: I) -> Self {
        Self::from_iter(value)
    }
}

impl<'a> FromIterator<(&'a str, &'a str)> for Filters {
    fn from_iter<I: IntoIterator<Item = (&'a str, &'a str)>>(iter: I) -> Self {
        let mut rv = Filters::default();
        for (regex, replacement) in iter {
            rv.add(regex, replacement);
        }
        rv
    }
}

impl Filters {
    /// Adds a simple regex with a replacement.
    pub(crate) fn add<S: Into<String>>(&mut self, regex: &str, replacement: S) {
        self.rules.push((
            Regex::new(regex).expect("invalid regex for snapshot filter rule"),
            replacement.into(),
        ));
    }

    /// Clears all filters.
    pub(crate) fn clear(&mut self) {
        self.rules.clear();
    }

    /// Applies all filters to the given snapshot.
    pub(crate) fn apply_to<'s>(&self, s: &'s str) -> Cow<'s, str> {
        let mut rv = Cow::Borrowed(s);

        for (regex, replacement) in &self.rules {
            match regex.replace_all(&rv, replacement) {
                Cow::Borrowed(_) => continue,
                Cow::Owned(value) => rv = Cow::Owned(value),
            };
        }

        rv
    }
}

/// Strips all ANSI escape sequences from the given string.
pub(crate) fn strip_ansi_escape_codes(s: &str) -> Cow<'_, str> {
    if s.contains('\x1b') {
        Cow::Owned(strip_ansi_escapes::strip_str(s))
    } else {
        Cow::Borrowed(s)
    }
}

#[test]
fn test_filters() {
    let mut filters = Filters::default();
    filters.add("\\bhello\\b", "[NAME]");
    filters.add("(a)", "[$1]");
    assert_eq!(
        filters.apply_to("hellohello hello abc"),
        "hellohello [NAME] [a]bc"
    );
}

#[test]
fn test_static_str_array_conversion() {
    let arr: [(&'static str, &'static str); 2] = [("a1", "b1"), ("a2", "b2")];
    let _ = Filters::from_iter(arr);
}

#[test]
fn test_vec_str_conversion() {
    let vec: Vec<(&str, &str)> = Vec::from([("a1", "b1"), ("a2", "b2")]);
    let _ = Filters::from(vec);
}

#[test]
fn test_strip_ansi_escape_codes_basic() {
    assert_eq!(strip_ansi_escape_codes("\x1b[31mhello\x1b[0m"), "hello");
}

#[test]
fn test_strip_ansi_escape_codes_no_codes() {
    let plain = "hello world";
    let result = strip_ansi_escape_codes(plain);
    assert_eq!(result, "hello world");
    // When there are no escape codes, the result should borrow (not allocate)
    assert!(matches!(result, Cow::Borrowed(_)));
}

#[test]
fn test_strip_ansi_escape_codes_multiple() {
    assert_eq!(
        strip_ansi_escape_codes("\x1b[1m\x1b[31mERROR\x1b[0m: something \x1b[32mfailed\x1b[0m"),
        "ERROR: something failed"
    );
}

#[test]
fn test_strip_ansi_escape_codes_256_color() {
    assert_eq!(strip_ansi_escape_codes("\x1b[38;5;196mred\x1b[0m"), "red");
}

#[test]
fn test_strip_ansi_escape_codes_rgb() {
    assert_eq!(
        strip_ansi_escape_codes("\x1b[38;2;255;0;0mred\x1b[0m"),
        "red"
    );
}
