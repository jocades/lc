#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[rustfmt::skip]
pub enum Token {
    Eof,
    // items
    Ident, Num, Unit,
    // delims
    Lam, Dot, Comma,
    Semi, LParen, RParen,
    //operators
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

pub type Span = std::ops::Range<usize>;

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
        self.start..self.cursor
    }

    pub fn lexeme(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.src[self.span()]) }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.cursor).copied()
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
        self.seek(|c| c.is_ascii_whitespace());
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
                self.seek(|c| c.is_ascii_alphanumeric() || c == b'_');
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
