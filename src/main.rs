#![allow(unused)]

use std::cell::RefCell;
use std::io::{self, BufRead, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[rustfmt::skip]
enum Token {
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
    True, False, And, Or,
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

#[derive(Default)]
struct Lexer<'a> {
    src: &'a [u8],
    start: usize,
    cursor: usize,
}

type Span = std::ops::Range<usize>;

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src: src.as_bytes(),
            start: 0,
            cursor: 0,
        }
    }

    fn spanned(self) -> SpannedIter<'a> {
        SpannedIter { lexer: self }
    }

    fn span(&self) -> Span {
        self.start..self.cursor
    }

    fn lexeme(&self) -> &str {
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

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.scan_token()
    }
}

struct SpannedIter<'a> {
    lexer: Lexer<'a>,
}

impl<'a> Iterator for SpannedIter<'a> {
    type Item = (Token, Span);

    fn next(&mut self) -> Option<Self::Item> {
        self.lexer.next().map(|token| (token, self.lexer.span()))
    }
}

use std::rc::Rc;
use std::{collections::HashMap, mem};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(u32);

pub struct Interner {
    map: HashMap<&'static str, Symbol>,
    vec: Vec<&'static str>,
    buf: String,
    full: Vec<String>,
}

impl Interner {
    pub fn with_capacity(cap: usize) -> Self {
        let cap = cap.next_power_of_two();
        Interner {
            map: HashMap::new(),
            vec: Vec::new(),
            buf: String::with_capacity(cap),
            full: Vec::new(),
        }
    }

    pub fn intern(&mut self, name: &str) -> Symbol {
        if let Some(&id) = self.map.get(name) {
            return id;
        }

        let name = self.alloc(name);
        let sym = Symbol(self.map.len() as u32);
        self.map.insert(name, sym);
        self.vec.push(name);

        sym
    }

    pub fn lookup(&self, sym: Symbol) -> &str {
        self.vec[sym.0 as usize]
    }

    pub fn alloc(&mut self, name: &str) -> &'static str {
        let cap = self.buf.capacity();
        if cap < self.buf.len() + name.len() {
            let new_cap = (cap.max(name.len()) + 1).next_power_of_two();
            let new_buf = String::with_capacity(new_cap);
            let old_buf = mem::replace(&mut self.buf, new_buf);
            self.full.push(old_buf);
        }

        let interned = {
            let start = self.buf.len();
            self.buf.push_str(name);
            &self.buf[start..]
        };

        unsafe { &*(interned as *const str) }
    }
}

#[derive(Debug)]
enum Lit {
    Unit,
    Int(i32),
    Bool(bool),
}

#[derive(Debug)]
enum Expr {
    Lit(Lit, Span),
    Var(Symbol, Span),
    Abs(Symbol, Rc<Expr>, Span),
    App(Rc<Expr>, Rc<Expr>, Span),
    Bin(Rc<Expr>, Token, Rc<Expr>, Span),
    Bind {
        is_recursive: bool,
        name: Symbol,
        init: Rc<Expr>,
        body: Rc<Expr>,
        span: Span,
    },
    Cond {
        cond: Rc<Expr>,
        then_branch: Rc<Expr>,
        else_branch: Rc<Expr>,
        span: Span,
    },
}

impl Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Lit(_, span) => span.clone(),
            Expr::Var(_, span) => span.clone(),
            Expr::Abs(_, _, span) => span.clone(),
            Expr::App(_, _, span) => span.clone(),
            Expr::Bin(_, _, _, span) => span.clone(),
            Expr::Bind { span, .. } => span.clone(),
            Expr::Cond { span, .. } => span.clone(),
        }
    }
}

/**
expr     := or ;
or       := and ('or' and)* ;
and      := eq ('and' eq)* ;
eq       := cmp (('==' | '!=') cmp)* ;
cmp      := term (('>' | '>=' | '<' | '<=') term)* ;
term     := factor (('+' | '-') factor)* ;
factor   := operand (('*' | '/') operand)* ;
operand  :=  bind | cond | match | app ;
bind     := 'let' 'rec'? ID+ '=' expr 'in' expr ;
cond     := 'if' expr 'then' expr 'else' expr ;
match    := 'match' expr 'with' '|'? case ('|' case)* ;
case     := pattern '->' expr ;
pattern  := pat_atom ('|' pat_atom)* ;
pat_atom := '_' | ID | LIT ;
app      := atom atom* ;
atom     := ID | LIT | '(' expr ')' | abs ;
abs      := '\' ID+ '.' expr ;
*/

fn parse(source: &str, interner: &mut Interner) -> Option<Expr> {
    Parser::new(Lexer::new(source), interner).parse()
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    interner: &'a mut Interner,
    current: Token,
    previous: Token,
}

impl<'a> Parser<'a> {
    fn new(lexer: Lexer<'a>, interner: &'a mut Interner) -> Self {
        Self {
            lexer,
            interner,
            current: Token::Eof,
            previous: Token::Eof,
        }
    }

    fn advance(&mut self) {
        self.previous = self.current;
        match self.lexer.next() {
            Some(token) => self.current = token,
            None => self.current = Token::Eof,
        }
    }

    fn matches(&mut self, token: Token) -> bool {
        if self.current == token {
            self.advance();
            return true;
        }
        false
    }

    fn intern_current(&mut self) -> Symbol {
        self.interner.intern(self.lexer.lexeme())
    }

    fn consume(&mut self, token: Token, reason: &str) {
        if self.current != token {
            panic!("{reason}");
        }
        self.advance();
    }

    fn consume_ident(&mut self, reason: &str) -> Symbol {
        if self.current != Token::Ident {
            panic!("{reason}");
        }
        let sym = self.intern_current();
        self.advance();
        sym
    }

    fn parse(&mut self) -> Option<Expr> {
        self.advance();
        (self.current != Token::Eof).then(|| self.expr())
    }

    fn expr(&mut self) -> Expr {
        self.eq()
    }

    // eq := cmp (('==' | '!=') cmp)* ;
    fn eq(&mut self) -> Expr {
        let mut expr = self.cmp();
        loop {
            match self.current {
                Token::EqEq | Token::BangEq => {
                    self.advance();
                    let op = self.previous;
                    let rhs = self.cmp();
                    let span = expr.span().start..rhs.span().end;
                    expr = Expr::Bin(Rc::new(expr), op, Rc::new(rhs), span);
                }
                _ => break,
            }
        }
        expr
    }

    // cmp := term (('>' | '>=' | '<' | '<=') term)* ;
    fn cmp(&mut self) -> Expr {
        let mut expr = self.term();
        loop {
            match self.current {
                Token::Gt | Token::GtEq | Token::Lt | Token::LtEq => {
                    self.advance();
                    let op = self.previous;
                    let rhs = self.term();
                    let span = expr.span().start..rhs.span().end;
                    expr = Expr::Bin(Rc::new(expr), op, Rc::new(rhs), span);
                }
                _ => break,
            }
        }
        expr
    }

    fn term(&mut self) -> Expr {
        let mut expr = self.factor();
        loop {
            match self.current {
                Token::Plus | Token::Minus => {
                    self.advance();
                    let op = self.previous;
                    let rhs = self.factor();
                    let span = expr.span().start..rhs.span().end;
                    expr = Expr::Bin(Rc::new(expr), op, Rc::new(rhs), span);
                }
                _ => break,
            }
        }
        expr
    }

    // factor := operand (('*' | '/') operand)* ;
    fn factor(&mut self) -> Expr {
        let mut expr = self.operand();
        loop {
            match self.current {
                Token::Star | Token::Slash => {
                    self.advance();
                    let op = self.previous;
                    let rhs = self.operand();
                    let span = expr.span().start..rhs.span().end;
                    expr = Expr::Bin(Rc::new(expr), op, Rc::new(rhs), span);
                }
                _ => break,
            }
        }
        expr
    }

    // operand := bind | cond | match | app ;
    fn operand(&mut self) -> Expr {
        match self.current {
            Token::Let => self.bind(),
            Token::If => self.cond(),
            Token::Match => self.match_(),
            _ => self.app(),
        }
    }

    // bind := 'let' 'rec'? ID+ '=' expr 'in' expr
    fn bind(&mut self) -> Expr {
        let start = self.lexer.span().start;
        self.advance(); // consume 'let'

        let is_recursive = self.matches(Token::Rec);
        let name = self.consume_ident("expected ident after 'let'");

        // sugar for binding abstractions:
        // let f a b = a + b in ...
        // let f = \a.\b.a+b in ...
        // non-abstraction bindings remain the same:
        // let x = 1 in ... *)

        let mut params = Vec::new();
        while self.current == Token::Ident {
            params.push(self.intern_current());
            self.advance();
        }
        self.consume(Token::Eq, "expected '=' after let binding");

        let mut init = self.expr();
        for param in params.into_iter().rev() {
            let span = start..init.span().end;
            init = Expr::Abs(param, Rc::new(init), span);
        }
        self.consume(Token::In, "expected 'in' after let initializer");

        let body = self.expr();
        Expr::Bind {
            is_recursive,
            name,
            init: Rc::new(init),
            body: Rc::new(body),
            span: start..self.lexer.span().end,
        }
    }

    fn cond(&mut self) -> Expr {
        let start = self.lexer.span().start;
        self.advance(); // consume 'if'
        let cond = self.expr();
        self.consume(Token::Then, "expected 'then' after condition");
        let then_branch = self.expr();
        self.consume(Token::Else, "expected 'else' after 'then' body");
        let else_branch = self.expr();
        let span = start..self.lexer.span().end;
        Expr::Cond {
            cond: Rc::new(cond),
            then_branch: Rc::new(then_branch),
            else_branch: Rc::new(else_branch),
            span,
        }
    }

    fn match_(&mut self) -> Expr {
        todo!()
    }

    // app := atom atom* ;
    fn app(&mut self) -> Expr {
        let mut expr = self.atom();
        loop {
            match self.current {
                Token::Ident
                | Token::Num
                | Token::True
                | Token::False
                | Token::LParen
                | Token::Lam => {
                    let rhs = self.atom();
                    let span = expr.span().start..rhs.span().end;
                    expr = Expr::App(Rc::new(expr), Rc::new(rhs), span);
                }
                _ => break,
            }
        }
        expr
    }

    // atom := ID | LIT | '(' expr ')' | abs ;
    fn atom(&mut self) -> Expr {
        let current = self.current;
        let expr = match current {
            Token::Ident => {
                let sym = self.intern_current();
                Expr::Var(sym, self.lexer.span())
            }

            Token::Num => {
                let int = self.lexer.lexeme().parse::<i32>().unwrap();
                Expr::Lit(Lit::Int(int), self.lexer.span())
            }

            Token::True => Expr::Lit(Lit::Bool(true), self.lexer.span()),
            Token::False => Expr::Lit(Lit::Bool(false), self.lexer.span()),

            Token::Unit => Expr::Lit(Lit::Unit, self.lexer.span()),

            Token::LParen => {
                self.advance();
                let expr = self.expr();
                self.consume(Token::RParen, "expected ')' after grouping");
                return expr;
            }

            Token::Lam => return self.abs(),

            _ => panic!("expected expression"),
        };
        self.advance();
        expr
    }

    /// abs := '\' ID+ '.' expr ;
    fn abs(&mut self) -> Expr {
        let start = self.lexer.span().start;
        self.advance(); // consume '\'

        let mut params = vec![self.consume_ident("expected ident after '\\'")];
        while self.current == Token::Ident {
            params.push(self.intern_current());
            self.advance();
        }
        self.consume(Token::Dot, "expected '.' after lambda param(s)");

        let mut body = self.expr();
        for param in params.into_iter().rev() {
            let span = start..body.span().end;
            body = Expr::Abs(param, Rc::new(body), span);
        }

        body
    }
}

type Env = HashMap<Symbol, Value>;

#[derive(Debug, Clone)]
enum Value {
    Unit,
    Int(i32),
    Bool(bool),
    Closure(Symbol, Rc<Expr>, RefCell<Env>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Unit => write!(f, "()"),
            Value::Int(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Closure(_, _, _) => write!(f, "<fn>"),
        }
    }
}

struct Context<'a> {
    env: &'a mut Env,
    interner: &'a Interner,
}

fn eval<'a>(cx: &mut Context<'a>, expr: Rc<Expr>) -> Result<Value, String> {
    let value = match expr.as_ref() {
        Expr::Lit(Lit::Unit, _) => Value::Unit,
        Expr::Lit(Lit::Int(int), _) => Value::Int(*int),
        Expr::Lit(Lit::Bool(b), _) => Value::Bool(*b),

        Expr::Var(sym, _) => match cx.env.get(sym) {
            Some(value) => value.clone(),
            None => return Err(format!("unbound variable {}", cx.interner.lookup(*sym))),
        },

        Expr::Abs(param, body, _) => {
            // This obviously performs poorly, copying the entire env hashmap...
            // But this eval is just for testing before compiling into bytecode.
            Value::Closure(*param, body.clone(), RefCell::new(cx.env.clone()))
        }

        Expr::App(lhs, rhs, _) => {
            let Value::Closure(param, body, mut env) = eval(cx, lhs.clone())? else {
                return Err("only closures are callable".into());
            };

            let arg = eval(cx, rhs.clone())?;

            let mut closure_env = env.borrow_mut();
            closure_env.insert(param, arg);

            let mut cx = Context {
                env: &mut closure_env,
                interner: cx.interner,
            };

            return eval(&mut cx, body.clone());
        }

        Expr::Bin(lhs, op, rhs, _) => {
            let (Value::Int(l), Value::Int(r)) = (eval(cx, lhs.clone())?, eval(cx, rhs.clone())?)
            else {
                return Err("operands must be numbers".into());
            };
            match op {
                Token::EqEq => Value::Bool(l == r),
                Token::BangEq => Value::Bool(l != r),
                Token::Plus => Value::Int(l + r),
                Token::Minus => Value::Int(l - r),
                Token::Star => Value::Int(l * r),
                Token::Slash => Value::Int(l / r),
                _ => unreachable!(),
            }
        }

        Expr::Bind {
            is_recursive,
            name,
            init,
            body,
            ..
        } => {
            let init = eval(cx, init.clone())?;
            cx.env.insert(*name, init);
            return eval(cx, body.clone());
        }

        Expr::Cond {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            return match eval(cx, cond.clone())? {
                Value::Bool(true) => eval(cx, then_branch.clone()),
                Value::Bool(false) => eval(cx, else_branch.clone()),
                _ => Err("only booleans are allowed in conditions".into()),
            };
        }
    };

    Ok(value)
}

fn main() {
    repl();
}

fn repl() {
    let mut interner = Interner::with_capacity(1024);
    let mut env = Env::new();

    let mut stdin = io::stdin().lock();
    let mut buf = String::new();
    loop {
        print!("λ> ");
        io::stdout().flush().unwrap();

        if stdin.read_line(&mut buf).unwrap() == 0 {
            break;
        }

        let Some(expr) = parse(&buf, &mut interner) else {
            buf.clear();
            continue;
        };

        println!("{expr:?}");

        let mut cx = Context {
            env: &mut env,
            interner: &interner,
        };

        match eval(&mut cx, Rc::new(expr)) {
            Ok(value) => println!("{value}"),
            Err(reason) => println!("runtime error: {reason}"),
        }

        println!("{env:?}");

        buf.clear();
    }
}
