#![allow(unused)]

use crate::lexer::Span;

pub struct Source<'a> {
    text: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> Source<'a> {
    pub fn new(text: &'a str) -> Self {
        let mut line_starts = vec![0];
        text.char_indices()
            .filter(|(i, c)| *c == '\n')
            .for_each(|(i, _)| line_starts.push(i + 1));
        Self { text, line_starts }
    }

    pub fn line_of(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(ln) => ln,
            Err(ln) => ln - 1,
        }
    }

    pub fn column_of(&self, offset: usize) -> usize {
        let ln = self.line_of(offset);
        offset - self.line_starts[ln]
    }

    pub fn line_col(&self, span: std::ops::Range<usize>) -> (usize, usize) {
        let ln = self.line_of(span.start);
        let col = span.start - self.line_starts[ln];
        (ln + 1, col + 1) // human readabale
    }

    pub fn line_range(&self, line_index: usize) -> std::ops::Range<usize> {
        let start = self.line_starts[line_index];
        let end = self
            .line_starts
            .get(line_index + 1)
            .copied()
            .unwrap_or(self.text.len());
        start..end
    }

    pub fn line_text(&self, line_index: usize) -> &'a str {
        let range = self.line_range(line_index);
        self.text[range].trim_end_matches('\n')
    }

    pub fn line_span(&self, span: Span) -> Span {
        let line = self.line_of(span.start);
        let range = self.line_range(line);
        Span::from(range)
    }
}
