use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use insta::SnapshotContents;
use proc_macro2::TokenTree;
use syn;
use syn::spanned::Spanned;

#[derive(Debug)]
pub struct InlineSnapshot {
    start: (usize, usize),
    end: (usize, usize),
    indentation: usize,
}

pub struct FilePatcher {
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
    pub fn open<P: AsRef<Path>>(p: P) -> Result<FilePatcher, Box<dyn Error>> {
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

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
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

    pub fn set_new_content(&mut self, id: usize, snapshot: &SnapshotContents) {
        let inline = &mut self.inline_snapshots[id];

        // find prefix and suffix
        let prefix = self.lines[inline.start.0][..inline.start.1].to_string();
        let suffix = self.lines[inline.end.0][inline.end.1..].to_string();

        // replace lines
        let snapshot_line_contents =
            vec![prefix, snapshot.to_inline(inline.indentation), suffix].join("");

        self.lines.splice(
            inline.start.0..=inline.end.0,
            snapshot_line_contents.lines().map(|l| l.to_string()),
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

        impl Visitor {
            pub fn scan_nested_macros(&mut self, tokens: &[TokenTree]) {
                if let Some(TokenTree::Ident(ref ident)) = tokens.get(0) {
                    if let Some(TokenTree::Punct(ref punct)) = tokens.get(1) {
                        if punct.as_char() == '!' {
                            if let Some(TokenTree::Group(ref group)) = tokens.get(2) {
                                let indentation = ident.span().start().column;
                                let tokens: Vec<_> = group.stream().into_iter().collect();
                                self.try_extract_snapshot(&tokens, indentation);
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

            pub fn try_extract_snapshot(
                &mut self,
                tokens: &[TokenTree],
                indentation: usize,
            ) -> bool {
                match &tokens[tokens.len() - 2] {
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
                    .tts
                    .clone()
                    .into_iter()
                    .last()
                    .map_or(start, |t| t.span().end().line);

                if start > self.0 || end < self.0 || i.path.segments.is_empty() {
                    return;
                }

                let tokens: Vec<_> = i.tts.clone().into_iter().collect();
                self.scan_nested_macros(&tokens);
            }

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

                // if we have under two tokens there is not much else we need to do
                let tokens: Vec<_> = i.tts.clone().into_iter().collect();
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
