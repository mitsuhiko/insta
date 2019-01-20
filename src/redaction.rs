use std::borrow::Cow;

use failure::Fail;
use pest::Parser;
use pest_derive::Parser;
use serde_yaml::{Number, Value};

#[derive(Fail, Debug)]
#[fail(display = "{}", _0)]
pub struct SelectorParseError(pest::error::Error<Rule>);

impl SelectorParseError {
    /// Return the column of where the error ocurred.
    #[allow(unused)]
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
pub enum Segment<'a> {
    Wildcard,
    Key(Cow<'a, str>),
    Index(i32),
    Range(Option<i32>, Option<i32>),
}

#[derive(Debug)]
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

    pub fn is_match(&self, path: &[Value]) -> bool {
        for selector in &self.selectors {
            if selector.len() != path.len() {
                return false;
            }
            for (segment, element) in selector.iter().zip(path.iter()) {
                let is_match = match *segment {
                    Segment::Wildcard => true,
                    Segment::Key(ref k) => element.as_str() == Some(&k),
                    Segment::Index(i) => element.as_i64() == Some(i64::from(i)),
                    Segment::Range(..) => unreachable!(),
                };
                if !is_match {
                    return false;
                }
            }
        }
        true
    }

    pub fn redact(&self, value: Value, redaction: &Value) -> Value {
        self.redact_impl(value, redaction, &mut vec![])
    }

    fn redact_impl(&self, value: Value, redaction: &Value, path: &mut Vec<Value>) -> Value {
        if self.is_match(&path) {
            redaction.clone()
        } else {
            match value {
                Value::Mapping(map) => Value::Mapping(
                    map.into_iter()
                        .map(|(key, value)| {
                            path.push(key.clone());
                            let new_value = self.redact_impl(value, redaction, path);
                            path.pop();
                            (key, new_value)
                        })
                        .collect(),
                ),
                Value::Sequence(seq) => Value::Sequence(
                    seq.into_iter()
                        .enumerate()
                        .map(|(idx, value)| {
                            path.push(Value::Number(Number::from(idx)));
                            let new_value = self.redact_impl(value, redaction, path);
                            path.pop();
                            new_value
                        })
                        .collect(),
                ),
                other => other,
            }
        }
    }
}
