use std::borrow::Cow;

use pest::Parser;
use pest_derive::Parser;

use crate::content::Content;

#[derive(Debug)]
pub struct SelectorParseError(pest::error::Error<Rule>);

impl SelectorParseError {
    /// Return the column of where the error ocurred.
    pub fn column(&self) -> usize {
        match self.0.line_col {
            pest::error::LineColLocation::Pos((_, col)) => col,
            pest::error::LineColLocation::Span((_, col), _) => col,
        }
    }
}

#[derive(Parser)]
#[grammar = "select_grammar.pest"]
pub struct SelectParser;

#[derive(Debug)]
pub enum PathItem {
    Content(Content),
    Field(&'static str),
    Index(u64, u64),
}

impl PathItem {
    fn as_str(&self) -> Option<&str> {
        match *self {
            PathItem::Content(ref content) => content.as_str(),
            PathItem::Field(s) => Some(s),
            PathItem::Index(..) => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match *self {
            PathItem::Content(ref content) => content.as_u64(),
            PathItem::Field(_) => None,
            PathItem::Index(idx, _) => Some(idx),
        }
    }

    fn range_check(&self, start: Option<i64>, end: Option<i64>) -> bool {
        fn expand_range(sel: i64, len: i64) -> i64 {
            if sel < 0 {
                (len + sel).max(0)
            } else {
                sel
            }
        }
        let (idx, len) = match *self {
            PathItem::Index(idx, len) => (idx as i64, len as i64),
            _ => return false,
        };
        match (start, end) {
            (None, None) => true,
            (None, Some(end)) => idx < expand_range(end, len),
            (Some(start), None) => idx >= expand_range(start, len),
            (Some(start), Some(end)) => {
                idx >= expand_range(start, len) && idx < expand_range(end, len)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Segment<'a> {
    Wildcard,
    Key(Cow<'a, str>),
    Index(u64),
    Range(Option<i64>, Option<i64>),
}

#[derive(Debug, Clone)]
pub struct Selector<'a> {
    selectors: Vec<Vec<Segment<'a>>>,
}

impl<'a> Selector<'a> {
    pub fn parse(selector: &'a str) -> Result<Selector<'a>, SelectorParseError> {
        let pair = SelectParser::parse(Rule::selectors, selector)
            .map_err(SelectorParseError)?
            .next()
            .unwrap();
        let mut rv = vec![];

        for selector_pair in pair.into_inner() {
            match selector_pair.as_rule() {
                Rule::EOI => break,
                other => assert_eq!(other, Rule::selector),
            }
            let mut segments = vec![];
            for segment_pair in selector_pair.into_inner() {
                segments.push(match segment_pair.as_rule() {
                    Rule::identity => continue,
                    Rule::wildcard => Segment::Wildcard,
                    Rule::key => Segment::Key(Cow::Borrowed(&segment_pair.as_str()[1..])),
                    Rule::subscript => {
                        let subscript_rule = segment_pair.into_inner().next().unwrap();
                        match subscript_rule.as_rule() {
                            Rule::int => Segment::Index(subscript_rule.as_str().parse().unwrap()),
                            Rule::string => {
                                let sq = subscript_rule.as_str();
                                let s = &sq[1..sq.len() - 1];
                                let mut was_backslash = false;
                                Segment::Key(if s.bytes().any(|x| x == b'\\') {
                                    Cow::Owned(
                                        s.chars()
                                            .filter_map(|c| {
                                                let rv = match c {
                                                    '\\' if !was_backslash => {
                                                        was_backslash = true;
                                                        return None;
                                                    }
                                                    other => other,
                                                };
                                                was_backslash = false;
                                                Some(rv)
                                            })
                                            .collect(),
                                    )
                                } else {
                                    Cow::Borrowed(s)
                                })
                            }
                            _ => unreachable!(),
                        }
                    }
                    Rule::full_range => Segment::Range(None, None),
                    Rule::range => {
                        let mut int_rule = segment_pair
                            .into_inner()
                            .map(|x| x.as_str().parse().unwrap());
                        Segment::Range(int_rule.next(), int_rule.next())
                    }
                    Rule::range_to => {
                        let int_rule = segment_pair.into_inner().next().unwrap();
                        Segment::Range(None, int_rule.as_str().parse().ok())
                    }
                    Rule::range_from => {
                        let int_rule = segment_pair.into_inner().next().unwrap();
                        Segment::Range(int_rule.as_str().parse().ok(), None)
                    }
                    _ => unreachable!(),
                });
            }
            rv.push(segments);
        }

        Ok(Selector { selectors: rv })
    }

    pub fn make_static(self) -> Selector<'static> {
        Selector {
            selectors: self
                .selectors
                .into_iter()
                .map(|parts| {
                    parts
                        .into_iter()
                        .map(|x| match x {
                            Segment::Key(x) => Segment::Key(Cow::Owned(x.into_owned())),
                            Segment::Index(x) => Segment::Index(x),
                            Segment::Wildcard => Segment::Wildcard,
                            Segment::Range(a, b) => Segment::Range(a, b),
                        })
                        .collect()
                })
                .collect(),
        }
    }

    pub fn is_match(&self, path: &[PathItem]) -> bool {
        for selector in &self.selectors {
            if selector.len() != path.len() {
                return false;
            }
            for (segment, element) in selector.iter().zip(path.iter()) {
                let is_match = match *segment {
                    Segment::Wildcard => true,
                    Segment::Key(ref k) => element.as_str() == Some(&k),
                    Segment::Index(i) => element.as_u64() == Some(i),
                    Segment::Range(start, end) => element.range_check(start, end),
                };
                if !is_match {
                    return false;
                }
            }
        }
        true
    }

    pub fn redact(&self, value: Content, redaction: &Content) -> Content {
        self.redact_impl(value, redaction, &mut vec![])
    }

    fn redact_seq(
        &self,
        seq: Vec<Content>,
        redaction: &Content,
        path: &mut Vec<PathItem>,
    ) -> Vec<Content> {
        let len = seq.len();
        seq.into_iter()
            .enumerate()
            .map(|(idx, value)| {
                path.push(PathItem::Index(idx as u64, len as u64));
                let new_value = self.redact_impl(value, redaction, path);
                path.pop();
                new_value
            })
            .collect()
    }

    fn redact_struct(
        &self,
        seq: Vec<(&'static str, Content)>,
        redaction: &Content,
        path: &mut Vec<PathItem>,
    ) -> Vec<(&'static str, Content)> {
        seq.into_iter()
            .map(|(key, value)| {
                path.push(PathItem::Field(key));
                let new_value = self.redact_impl(value, redaction, path);
                path.pop();
                (key, new_value)
            })
            .collect()
    }

    fn redact_impl(
        &self,
        value: Content,
        redaction: &Content,
        path: &mut Vec<PathItem>,
    ) -> Content {
        if self.is_match(&path) {
            redaction.clone()
        } else {
            match value {
                Content::Map(map) => Content::Map(
                    map.into_iter()
                        .map(|(key, value)| {
                            path.push(PathItem::Content(key.clone()));
                            let new_value = self.redact_impl(value, redaction, path);
                            path.pop();
                            (key, new_value)
                        })
                        .collect(),
                ),
                Content::Seq(seq) => Content::Seq(self.redact_seq(seq, redaction, path)),
                Content::Tuple(seq) => Content::Tuple(self.redact_seq(seq, redaction, path)),
                Content::TupleStruct(name, seq) => {
                    Content::TupleStruct(name, self.redact_seq(seq, redaction, path))
                }
                Content::TupleVariant(name, variant_index, variant, seq) => Content::TupleVariant(
                    name,
                    variant_index,
                    variant,
                    self.redact_seq(seq, redaction, path),
                ),
                Content::Struct(name, seq) => {
                    Content::Struct(name, self.redact_struct(seq, redaction, path))
                }
                Content::StructVariant(name, variant_index, variant, seq) => {
                    Content::StructVariant(
                        name,
                        variant_index,
                        variant,
                        self.redact_struct(seq, redaction, path),
                    )
                }
                other => other,
            }
        }
    }
}

#[test]
fn test_range_checks() {
    assert_eq!(PathItem::Index(0, 10).range_check(None, Some(-1)), true);
    assert_eq!(PathItem::Index(9, 10).range_check(None, Some(-1)), false);
    assert_eq!(PathItem::Index(0, 10).range_check(Some(1), Some(-1)), false);
    assert_eq!(PathItem::Index(1, 10).range_check(Some(1), Some(-1)), true);
    assert_eq!(PathItem::Index(9, 10).range_check(Some(1), Some(-1)), false);
    assert_eq!(PathItem::Index(0, 10).range_check(Some(1), None), false);
    assert_eq!(PathItem::Index(1, 10).range_check(Some(1), None), true);
    assert_eq!(PathItem::Index(9, 10).range_check(Some(1), None), true);
}
