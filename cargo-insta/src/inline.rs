use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use failure::Error;
use proc_macro2::TokenTree;
use syn;
use syn::spanned::Spanned;

#[derive(Debug)]
pub struct InlineSnapshot {
    start: (usize, usize),
    end: (usize, usize),
    indentation: usize,
}

#[derive(Debug)]
pub struct FilePatcher {
    filename: PathBuf,
    lines: Vec<String>,
    newline: &'static str,
    source: syn::File,
    inline_snapshots: Vec<InlineSnapshot>,
}

impl FilePatcher {
    pub fn open<P: AsRef<Path>>(p: P) -> Result<FilePatcher, Error> {
        let filename = p.as_ref().to_path_buf();
        let contents = fs::read_to_string(p)?;
        let source = syn::parse_file(&contents)?;
        let mut line_iter = contents.lines().peekable();
        let newline = if let Some(line) = line_iter.peek() {
            match contents.as_bytes().get(line.len() + 1) {
                Some(b'\r') => &"\r\n",
                _ => &"\n",
            }
        } else {
            &"\n"
        };
        let lines: Vec<String> = line_iter.map(|x| x.into()).collect();
        Ok(FilePatcher {
            filename,
            source,
            newline,
            lines,
            inline_snapshots: vec![],
        })
    }

    pub fn save(&self) -> Result<(), Error> {
        let mut f = fs::File::create(&self.filename)?;
        for line in &self.lines {
            writeln!(&mut f, "{}", line)?;
        }
        Ok(())
    }

    pub fn add_snapshot_macro(&mut self, line: usize) {
        match self.find_snapshot_macro(line) {
            Some(snapshot) => {
                assert!(self
                    .inline_snapshots
                    .last()
                    .map_or(true, |x| x.end.0 <= line));
                self.inline_snapshots.push(snapshot)
            }
            None => panic!("Could not find snapshot in line {}", line),
        }
    }

    pub fn get_new_line(&self, id: usize) -> usize {
        self.inline_snapshots[id].start.0 + 1
    }

    pub fn set_new_content(&mut self, id: usize, snapshot: &str) {
        let inline = &mut self.inline_snapshots[id];
        let old_lines = inline.end.0 - inline.start.0 + 1;

        // find prefix and suffix
        let prefix: String = self.lines[inline.start.0]
            .chars()
            .take(inline.start.1)
            .collect();
        let suffix: String = self.lines[inline.end.0]
            .chars()
            .skip(inline.end.1)
            .collect();

        // replace lines
        let mut new_lines: Vec<_> = snapshot.lines().map(Cow::Borrowed).collect();
        if new_lines.is_empty() {
            new_lines.push(Cow::Borrowed(""));
        }

        // if we have more than one line we want to change into the block
        // representation mode
        if new_lines.len() > 1 || snapshot.contains('┇') {
            new_lines.insert(0, Cow::Borrowed(""));
            if inline.indentation > 0 {
                for (idx, line) in new_lines.iter_mut().enumerate() {
                    if idx == 0 {
                        continue;
                    }
                    *line = Cow::Owned(format!(
                        "{c: >width$}{line}",
                        c = "⋮",
                        width = inline.indentation,
                        line = line
                    ));
                }
                new_lines.push(Cow::Owned(format!(
                    "{c: >width$}",
                    c = " ",
                    width = inline.indentation
                )));
            } else {
                new_lines.push(Cow::Borrowed(""));
            }
        }

        let (quote_start, quote_end) =
            if new_lines.len() > 1 || new_lines[0].contains(&['\\', '"'][..]) {
                ("r###\"", "\"###")
            } else {
                ("\"", "\"")
            };
        let line_count_diff = new_lines.len() as i64 - old_lines as i64;

        self.lines.splice(
            inline.start.0..=inline.end.0,
            new_lines.iter().enumerate().map(|(idx, line)| {
                let mut rv = String::new();
                if idx == 0 {
                    rv.push_str(&prefix);
                    rv.push_str(quote_start);
                }
                rv.push_str(&line);
                if idx + 1 == new_lines.len() {
                    rv.push_str(quote_end);
                    rv.push_str(&suffix);
                }
                rv
            }),
        );

        for inl in &mut self.inline_snapshots[id..] {
            inl.start.0 = (inl.start.0 as i64 + line_count_diff) as usize;
            inl.end.0 = (inl.end.0 as i64 + line_count_diff) as usize;
        }
    }

    fn find_snapshot_macro(&self, line: usize) -> Option<InlineSnapshot> {
        struct Visitor(usize, Option<InlineSnapshot>);

        impl<'ast> syn::visit::Visit<'ast> for Visitor {
            fn visit_macro(&mut self, i: &'ast syn::Macro) {
                let indentation = i.span().start().column;
                let start = i.span().start().line;
                let end = i
                    .tts
                    .clone()
                    .into_iter()
                    .last()
                    .map_or(start, |t| t.span().end().line);

                if start > self.0 || end < self.0 || i.path.segments.is_empty() {
                    return;
                }

                let tokens: Vec<_> = i.tts.clone().into_iter().collect();
                if tokens.len() < 2 {
                    return;
                }

                match &tokens[tokens.len() - 2] {
                    TokenTree::Punct(ref punct) if punct.as_char() == '@' => {}
                    _ => return,
                }

                let (start, end) = match &tokens[tokens.len() - 1] {
                    TokenTree::Literal(lit) => {
                        let span = lit.span();
                        (
                            (span.start().line - 1, span.start().column),
                            (span.end().line - 1, span.end().column),
                        )
                    }
                    _ => return,
                };

                self.1 = Some(InlineSnapshot {
                    start,
                    end,
                    indentation,
                });
            }
        }

        let mut visitor = Visitor(line, None);
        syn::visit::visit_file(&mut visitor, &self.source);
        visitor.1
    }
}
