/*!
A recursive descent parser.

Language grammar:
```txt
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
atom     := ID | LIT | '(' expr ')' | fun ;
fun      := '\' ID+ '.' expr ;
```
*/
use crate::ast::{Ast, Expr, ExprId, Lit};
use crate::lexer::{Lexer, Token};
use crate::{Interner, Symbol};

pub fn parse(source: &str, interner: &mut Interner) -> Option<(Ast, ExprId)> {
    Parser::new(Lexer::new(source), interner).parse()
}

pub struct Parser<'a> {
    ast: Ast,
    lexer: Lexer<'a>,
    interner: &'a mut Interner,
    current: Token,
    previous: Token,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer<'a>, interner: &'a mut Interner) -> Self {
        Self {
            ast: Ast::default(),
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

    pub fn parse(mut self) -> Option<(Ast, ExprId)> {
        self.advance();
        if self.current == Token::Eof {
            return None;
        }
        let expr = self.expr();
        Some((self.ast, expr))
    }

    fn expr(&mut self) -> ExprId {
        self.eq()
    }

    fn bin(&mut self, sub: fn(&mut Self) -> ExprId, ops: &[Token]) -> ExprId {
        let mut expr = sub(self);
        loop {
            if !ops.contains(&self.current) {
                break;
            }
            self.advance();
            let op = self.previous;
            let rhs = sub(self);
            let span = self.ast.join_span(expr, rhs);
            expr = self.ast.alloc(Expr::Bin(expr, op, rhs), span)
        }
        expr
    }

    // eq := cmp (('==' | '!=') cmp)* ;
    fn eq(&mut self) -> ExprId {
        self.bin(Self::cmp, &[Token::EqEq, Token::BangEq])
    }

    // cmp := term (('>' | '>=' | '<' | '<=') term)* ;
    fn cmp(&mut self) -> ExprId {
        self.bin(
            Self::term,
            &[Token::Gt, Token::GtEq, Token::Lt, Token::LtEq],
        )
    }

    // term := factor (('+' | '-') factor)* ;
    fn term(&mut self) -> ExprId {
        self.bin(Self::factor, &[Token::Plus, Token::Minus])
    }

    // factor := operand (('*' | '/') operand)* ;
    fn factor(&mut self) -> ExprId {
        self.bin(Self::operand, &[Token::Star, Token::Slash])
    }

    // operand := bind | cond | match | app ;
    fn operand(&mut self) -> ExprId {
        match self.current {
            Token::Let => self.bind(),
            Token::If => self.cond(),
            Token::Match => self.match_(),
            _ => self.app(),
        }
    }

    // bind := 'let' 'rec'? ID+ '=' expr 'in' expr
    fn bind(&mut self) -> ExprId {
        let start = self.lexer.span();
        self.advance(); // consume 'let'

        let is_recursive = self.matches(Token::Rec);
        let name = self.consume_ident("expected ident after 'let'");

        // sugar for binding abstractions:
        // let f a b = a + b in ...
        // let f = \a.\b.a+b in ...
        // non-abstraction bindings remain the same:
        // let x = 1 in ...

        let mut params = Vec::new();
        while self.current == Token::Ident {
            params.push(self.intern_current());
            self.advance();
        }
        self.consume(Token::Eq, "expected '=' after let binding");

        let mut init = self.expr();
        for param in params.into_iter().rev() {
            let span = start | self.ast.span(init);
            init = self.ast.alloc(Expr::Fun(param, init), span);
        }
        self.consume(Token::In, "expected 'in' after let initializer");

        let body = self.expr();
        let span = start | self.lexer.span();

        self.ast.alloc(
            Expr::Bind {
                is_recursive,
                name,
                init,
                body,
            },
            span,
        )
    }

    fn cond(&mut self) -> ExprId {
        let start = self.lexer.span();
        self.advance(); // consume 'if'
        let cond = self.expr();
        self.consume(Token::Then, "expected 'then' after condition");
        let then_branch = self.expr();
        self.consume(Token::Else, "expected 'else' after 'then' body");
        let else_branch = self.expr();
        let span = start | self.lexer.span();
        self.ast.alloc(
            Expr::Cond {
                cond,
                then_branch,
                else_branch,
            },
            span,
        )
    }

    fn match_(&mut self) -> ExprId {
        todo!()
    }

    // app := atom atom* ;
    fn app(&mut self) -> ExprId {
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
                    let span = self.ast.join_span(expr, rhs);
                    expr = self.ast.alloc(Expr::App(expr, rhs), span);
                }
                _ => break,
            }
        }
        expr
    }

    // atom := ID | LIT | '(' expr ')' | fun ;
    fn atom(&mut self) -> ExprId {
        let (expr, span) = match self.current {
            Token::Ident => {
                let sym = self.intern_current();
                (Expr::Var(sym), self.lexer.span())
            }

            Token::Num => {
                let int = self.lexer.lexeme().parse::<i32>().unwrap();
                (Expr::Lit(Lit::Int(int)), self.lexer.span())
            }

            Token::True => (Expr::Lit(Lit::Bool(true)), self.lexer.span()),
            Token::False => (Expr::Lit(Lit::Bool(false)), self.lexer.span()),

            Token::Unit => (Expr::Lit(Lit::Unit), self.lexer.span()),

            Token::LParen => {
                self.advance();
                let expr = self.expr();
                self.consume(Token::RParen, "expected ')' after grouping");
                return expr;
            }

            Token::Lam => return self.fun(),

            _ => panic!("expected expression"),
        };

        self.advance();
        self.ast.alloc(expr, span)
    }

    // fun := '\' ID+ '.' expr ;
    fn fun(&mut self) -> ExprId {
        let start = self.lexer.span();
        self.advance(); // consume '\'

        let mut params = vec![self.consume_ident("expected ident after '\\'")];
        while self.current == Token::Ident {
            params.push(self.intern_current());
            self.advance();
        }
        self.consume(Token::Dot, "expected '.' after lambda param(s)");

        let mut body = self.expr();
        for param in params.into_iter().rev() {
            let span = start | self.ast.span(body);
            body = self.ast.alloc(Expr::Fun(param, body), span);
        }

        body
    }
}
