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
    source: syn::File,
    inline_snapshots: Vec<InlineSnapshot>,
}

impl FilePatcher {
    pub fn open<P: AsRef<Path>>(p: P) -> Result<FilePatcher, Error> {
        let filename = p.as_ref().to_path_buf();
        let contents = fs::read_to_string(p)?;
        let source = syn::parse_file(&contents)?;
        let lines: Vec<String> = contents.lines().map(|x| x.into()).collect();
        Ok(FilePatcher {
            filename,
            source,
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

        // find prefix and suffix on the first and last lines
        let prefix = self.lines[inline.start.0][..inline.start.1].to_string();
        let suffix = self.lines[inline.end.0][inline.end.1..].to_string();

        // replace lines
        let snapshot_line_contents = vec![
            prefix,
            denomalize_inline_snapshot(snapshot, inline.indentation),
            suffix,
        ]
        .join("");

        self.lines.splice(
            inline.start.0..=inline.end.0,
            snapshot_line_contents.lines().map(|l| l.into()),
        );

        // update other snapshot locations
        let old_lines_count = inline.end.0 - inline.start.0 + 1;
        let line_count_diff = snapshot_line_contents.lines().count() - old_lines_count;
        for inl in &mut self.inline_snapshots[id..] {
            inl.start.0 += line_count_diff;
            inl.end.0 += line_count_diff;
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

// from a snapshot to a string we want to write back
fn denomalize_inline_snapshot(snapshot: &str, indentation: usize) -> String {
    // could potentially implement as impl From<Snapshot> -> String

    let mut out = String::new();
    let is_escape = snapshot.lines().count() > 1 || snapshot.contains(&['\\', '"'][..]);

    out.push_str(if is_escape { "r###\"" } else { "\"" });
    // if we have more than one line we want to change into the block
    // representation mode
    if snapshot.lines().count() > 1 {
        out.push_str("\n");
        out.extend(
            snapshot
                .lines()
                .map(|l| format!("{c: >width$}{l}\n", c = "", width = indentation, l = l)),
        );
        out.push_str(&format!("{c: >width$}", c = "", width = indentation));
    } else {
        out.push_str(snapshot);
    }

    out.push_str(if is_escape { "\"###" } else { "\"" });

    out
}

#[test]
fn test_denormalize_inline_snapshot() {
    let t = &"
a
b"[1..];
    assert_eq!(
        denomalize_inline_snapshot(t, 0),
        "r###\"
a
b
\"###"
    );

    let t = &"
a
b"[1..];
    assert_eq!(
        denomalize_inline_snapshot(t, 4),
        "r###\"
    a
    b
    \"###"
    );

    let t = &"
    a
    b"[1..];
    assert_eq!(
        denomalize_inline_snapshot(t, 0),
        "r###\"
    a
    b
\"###"
    );

    let t = "ab";
    assert_eq!(denomalize_inline_snapshot(t, 0), r##""ab""##);
}
