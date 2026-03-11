use std::{fmt, ops};

/// A `Token` is a copyable enum, with no data attached, to make it easy to match
/// against and pass around.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[rustfmt::skip]
pub enum Token {
    Eof,
    // items
    Ident, Num, Unit,
    // delims
    Lam, Dot, Comma,
    Semi, LParen, RParen,
    // operators
    Plus, Minus, Star, Slash,
    Eq, EqEq, Bang, BangEq,
    Gt, GtEq, Lt, LtEq,
    Arrow, Pipe,
    // keywords
    True, False,
    And, Or,
    If, Then, Else,
    Let, Rec, In,
    Match, With,
}

fn lookup_ident(lexeme: &str) -> Token {
    match lexeme {
        "true" => Token::True,
        "false" => Token::False,
        "and" => Token::And,
        "or" => Token::Or,
        "if" => Token::If,
        "then" => Token::Then,
        "else" => Token::Else,
        "let" => Token::Let,
        "rec" => Token::Rec,
        "in" => Token::In,
        "match" => Token::Match,
        "with" => Token::With,
        _ => Token::Ident,
    }
}

/// A wrapper for [std::ops::Range<usize>] to make it copyable and extensible.
///
/// A span can be constructed from, and turned into, a `std::ops::Range<usize>`.
/// To join spans use the [Span::union] method or the overloaded `bitwise or`.
/// ```
/// let first = Span::from(0..2); // {start: 0, end: 2}
/// let second = Span::from(4..8); // {start: 4, end: 8}
/// let span = first | second; // {start: 0, end: 8} same as `first.union(second)`
/// assert_eq!(span, Span { start: 0, end: 8});
/// ```
#[derive(Clone, Copy)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn range(&self) -> ops::Range<usize> {
        self.start..self.end
    }

    pub fn union(&self, other: Span) -> Span {
        (self.start..other.end).into()
    }
}

impl ops::BitOr for Span {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl From<ops::Range<usize>> for Span {
    fn from(range: ops::Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.range().fmt(f)
    }
}

/// Iterates over the source text and yields [Token]s.
///
/// The `Parser` uses [Lexer::next], [Lexer::span] and [Lexer::lexeme] to retrieve
/// all the information needed to construct the `AST`.
#[derive(Default)]
pub struct Lexer<'a> {
    src: &'a [u8],
    start: usize,
    cursor: usize,
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.scan_token()
    }
}

#[allow(dead_code)]
pub struct SpannedIter<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Iterator for SpannedIter<'a> {
    type Item = (Token, Span);

    fn next(&mut self) -> Option<Self::Item> {
        self.lexer.next().map(|token| (token, self.lexer.span()))
    }
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            start: 0,
            cursor: 0,
        }
    }

    #[allow(dead_code)]
    pub fn spanned(self) -> SpannedIter<'a> {
        SpannedIter { lexer: self }
    }

    pub fn span(&self) -> Span {
        (self.start..self.cursor).into()
    }

    pub fn lexeme(&self) -> &'a str {
        unsafe { str::from_utf8_unchecked(&self.src[self.start..self.cursor]) }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.cursor).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.src.get(self.cursor + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        self.peek().map(|c| {
            self.cursor += 1;
            c
        })
    }

    fn seek(&mut self, pred: impl Fn(u8) -> bool) {
        while self.peek().is_some_and(&pred) {
            self.advance();
        }
    }

    fn matches(&mut self, m: u8) -> bool {
        if self.peek() == Some(m) {
            self.advance();
            return true;
        }
        false
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_whitespace() => {
                    self.seek(|c| c.is_ascii_whitespace());
                }
                Some(b'/') if self.peek2() == Some(b'/') => {
                    self.seek(|c| c != b'\n');
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn scan_token(&mut self) -> Option<Token> {
        self.skip_whitespace();
        self.start = self.cursor;

        let Some(c) = self.advance() else {
            return None;
        };

        let token = match c {
            // delims
            b'\\' => Token::Lam,
            b'.' => Token::Dot,
            b',' => Token::Comma,
            b';' => Token::Semi,
            b'(' if self.matches(b')') => Token::Unit,
            b'(' => Token::LParen,
            b')' => Token::RParen,
            // operators
            b'+' => Token::Plus,
            b'-' if self.matches(b'>') => Token::Arrow,
            b'-' => Token::Minus,
            b'*' => Token::Star,
            b'/' => Token::Slash,
            b'=' if self.matches(b'=') => Token::EqEq,
            b'=' => Token::Eq,
            b'!' if self.matches(b'=') => Token::BangEq,
            b'!' => Token::Bang,
            b'>' if self.matches(b'=') => Token::GtEq,
            b'>' => Token::Gt,
            b'<' if self.matches(b'=') => Token::LtEq,
            b'<' => Token::Lt,
            b'|' => Token::Pipe,
            // items
            c if c.is_ascii_alphabetic() || c == b'_' => {
                self.seek(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'\'');
                lookup_ident(self.lexeme())
            }
            c if c.is_ascii_digit() => {
                self.seek(|c| c.is_ascii_digit());
                Token::Num
            }
            _ => panic!("unexpectd character {c}"),
        };

        Some(token)
    }
}
