use std::borrow::Cow;

use regex::Regex;

/// Represents stored filters.
#[derive(Debug, Default, Clone)]
#[cfg_attr(docsrs, doc(cfg(feature = "filters")))]
pub struct Filters {
    rules: Vec<(Regex, String)>,
}

impl<'a> From<Vec<(&'a str, &'a str)>> for Filters {
    fn from(value: Vec<(&'a str, &'a str)>) -> Filters {
        let mut rv = Filters::default();
        for (regex, replacement) in value {
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
