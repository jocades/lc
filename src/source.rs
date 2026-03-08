#![allow(unused)]

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
}
