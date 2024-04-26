use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use insta::_cargo_insta_support::SnapshotContents;
use proc_macro2::TokenTree;

use syn::spanned::Spanned;

#[derive(Debug)]
struct InlineSnapshot {
    start: (usize, usize),
    end: (usize, usize),
    indentation: usize,
}

pub(crate) struct FilePatcher {
    filename: PathBuf,
    lines: Vec<String>,
    source: syn::File,
    inline_snapshots: Vec<InlineSnapshot>,
}

impl fmt::Debug for FilePatcher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("FilePatcher")
            .field("filename", &self.filename)
            .field("inline_snapshots", &self.inline_snapshots)
            .finish()
    }
}

impl FilePatcher {
    pub(crate) fn open(p: &Path) -> Result<FilePatcher, Box<dyn Error>> {
        let filename = p.to_path_buf();
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

    pub(crate) fn save(&self) -> Result<(), Box<dyn Error>> {
        // We use a temp file and then atomically rename to prevent a
        // file watcher restarting the process midway through the write.
        let mut temp_file = tempfile::Builder::new()
            .suffix(".snap.tmp")
            .tempfile_in(self.filename.parent().ok_or("Parent directory not found")?)?;

        for line in &self.lines {
            writeln!(temp_file, "{}", line)?;
        }

        temp_file.flush()?;
        fs::rename(temp_file.path(), &self.filename)?;

        Ok(())
    }

    pub(crate) fn add_snapshot_macro(&mut self, line: usize) -> bool {
        match self.find_snapshot_macro(line) {
            Some(snapshot) => {
                // this can happen if multiple snapshots were added in one
                // iteration of a loop.  In that case we want to ignore the
                // duplicate
                //
                // See https://github.com/mitsuhiko/insta/issues/340
                if self
                    .inline_snapshots
                    .last()
                    .map_or(false, |x| x.end.0 > line)
                {
                    return false;
                }
                self.inline_snapshots.push(snapshot);
                true
            }
            None => false,
        }
    }

    pub(crate) fn get_new_line(&self, id: usize) -> usize {
        self.inline_snapshots[id].start.0 + 1
    }

    pub(crate) fn set_new_content(&mut self, id: usize, snapshot: &SnapshotContents) {
        let inline = &mut self.inline_snapshots[id];

        // find prefix and suffix on the first and last lines
        let prefix: String = self.lines[inline.start.0]
            .chars()
            .take(inline.start.1)
            .collect();
        let suffix: String = self.lines[inline.end.0]
            .chars()
            .skip(inline.end.1)
            .collect();

        // replace lines
        let snapshot_line_contents = [
            prefix,
            snapshot.to_inline(inline.indentation).unwrap(),
            suffix,
        ]
        .join("");

        self.lines.splice(
            inline.start.0..=inline.end.0,
            snapshot_line_contents.lines().map(|l| l.to_string()),
        );

        // update other snapshot locations
        let old_lines_count = inline.end.0 - inline.start.0 + 1;
        let line_count_diff =
            (snapshot_line_contents.lines().count() as isize) - (old_lines_count as isize);
        for inl in &mut self.inline_snapshots[id..] {
            inl.start.0 = ((inl.start.0 as isize) + line_count_diff) as usize;
            inl.end.0 = ((inl.end.0 as isize) + line_count_diff) as usize;
        }
    }

    fn find_snapshot_macro(&self, line: usize) -> Option<InlineSnapshot> {
        struct Visitor(usize, Option<InlineSnapshot>);

        fn scan_for_path_start(tokens: &[TokenTree], pos: usize) -> usize {
            let mut rev_tokens = tokens[..=pos].iter().rev();
            let mut start = rev_tokens.next().unwrap();
            loop {
                if let Some(TokenTree::Punct(ref punct)) = rev_tokens.next() {
                    if punct.as_char() == ':' {
                        if let Some(TokenTree::Punct(ref punct)) = rev_tokens.next() {
                            if punct.as_char() == ':' {
                                if let Some(ident @ TokenTree::Ident(_)) = rev_tokens.next() {
                                    start = ident;
                                    continue;
                                }
                            }
                        }
                    }
                }
                break;
            }
            start.span().start().column
        }

        impl Visitor {
            fn scan_nested_macros(&mut self, tokens: &[TokenTree]) {
                for idx in 0..tokens.len() {
                    if let Some(TokenTree::Ident(_)) = tokens.get(idx) {
                        if let Some(TokenTree::Punct(ref punct)) = tokens.get(idx + 1) {
                            if punct.as_char() == '!' {
                                if let Some(TokenTree::Group(ref group)) = tokens.get(idx + 2) {
                                    let indentation = scan_for_path_start(tokens, idx);
                                    let tokens: Vec<_> = group.stream().into_iter().collect();
                                    self.try_extract_snapshot(&tokens, indentation);
                                }
                            }
                        }
                    }
                }

                for token in tokens {
                    // recurse into groups
                    if let TokenTree::Group(group) = token {
                        let tokens: Vec<_> = group.stream().into_iter().collect();
                        self.scan_nested_macros(&tokens);
                    }
                }
            }

            fn try_extract_snapshot(&mut self, tokens: &[TokenTree], indentation: usize) -> bool {
                if tokens.len() < 2 {
                    return false;
                }

                match tokens[tokens.len() - 2] {
                    TokenTree::Punct(ref punct) if punct.as_char() == '@' => {}
                    _ => {
                        return false;
                    }
                }

                let (start, end) = match &tokens[tokens.len() - 1] {
                    TokenTree::Literal(lit) => {
                        let span = lit.span();
                        (
                            (span.start().line - 1, span.start().column),
                            (span.end().line - 1, span.end().column),
                        )
                    }
                    _ => return false,
                };

                self.1 = Some(InlineSnapshot {
                    start,
                    end,
                    indentation,
                });
                true
            }
        }

        impl<'ast> syn::visit::Visit<'ast> for Visitor {
            fn visit_attribute(&mut self, i: &'ast syn::Attribute) {
                let start = i.span().start().line;
                let end = i
                    .tokens
                    .clone()
                    .into_iter()
                    .last()
                    .map_or(start, |t| t.span().end().line);

                if start > self.0 || end < self.0 || i.path.segments.is_empty() {
                    return;
                }

                let tokens: Vec<_> = i.tokens.clone().into_iter().collect();
                self.scan_nested_macros(&tokens);
            }

            fn visit_macro(&mut self, i: &'ast syn::Macro) {
                let indentation = i.span().start().column;
                let start = i.span().start().line;
                let end = i
                    .tokens
                    .clone()
                    .into_iter()
                    .last()
                    .map_or(start, |t| t.span().end().line);

                if start > self.0 || end < self.0 || i.path.segments.is_empty() {
                    return;
                }

                // if we have under two tokens there is not much else we need to do
                let tokens: Vec<_> = i.tokens.clone().into_iter().collect();
                if tokens.len() < 2 {
                    return;
                }

                if !self.try_extract_snapshot(&tokens, indentation) {
                    // if we can't extract a snapshot here we want to scan for nested
                    // macros.  These are just represented as unparsed tokens in a
                    // token stream.
                    self.scan_nested_macros(&tokens);
                }
            }
        }

        let mut visitor = Visitor(line, None);
        syn::visit::visit_file(&mut visitor, &self.source);
        visitor.1
    }
}
