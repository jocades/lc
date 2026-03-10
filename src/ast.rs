use crate::Symbol;
use crate::interner::Interner;
use crate::lexer::{Span, Token};
use crate::resolver::Local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExprId(pub u32);

#[derive(Default)]
pub struct Ast {
    pub nodes: Vec<Expr>,
    pub spans: Vec<Span>,
}

impl Ast {
    pub fn alloc(&mut self, expr: Expr, span: Span) -> ExprId {
        let id = ExprId(self.nodes.len() as u32);
        self.nodes.push(expr);
        self.spans.push(span);
        id
    }

    pub fn get(&self, id: ExprId) -> &Expr {
        &self.nodes[id.0 as usize]
    }

    pub fn span(&self, id: ExprId) -> Span {
        self.spans[id.0 as usize]
    }

    pub fn join_span(&self, a: ExprId, b: ExprId) -> Span {
        self.span(a) | self.span(b)
    }
}

impl std::ops::Index<ExprId> for Ast {
    type Output = Expr;

    fn index(&self, index: ExprId) -> &Self::Output {
        self.get(index)
    }
}

#[derive(Debug)]
pub enum Lit {
    Unit,
    Int(i32),
    Bool(bool),
}

#[derive(Debug)]
pub enum Expr {
    Lit(Lit),
    Var(Symbol),
    Abs(Symbol, ExprId),
    App(ExprId, ExprId),
    Bin(ExprId, Token, ExprId),
    Bind {
        is_recursive: bool,
        name: Symbol,
        init: ExprId,
        body: ExprId,
    },
    Cond {
        cond: ExprId,
        then_branch: ExprId,
        else_branch: ExprId,
    },
}

impl Ast {
    pub fn pretty(&self, root: ExprId, interner: &Interner, locals: &[Option<Local>]) -> String {
        let mut out = String::new();
        self.pretty_expr(root, interner, locals, 0, &mut out);
        out
    }

    fn pretty_expr(
        &self,
        expr: ExprId,
        interner: &Interner,
        locals: &[Option<Local>],
        indent: usize,
        out: &mut String,
    ) {
        use std::fmt::Write;
        let pad = "  ".repeat(indent);

        match &self[expr] {
            Expr::Lit(Lit::Unit) => _ = writeln!(out, "{pad}(lit ())"),
            Expr::Lit(Lit::Int(n)) => _ = writeln!(out, "{pad}(lit {n})"),
            Expr::Lit(Lit::Bool(b)) => _ = writeln!(out, "{pad}(lit {b})"),
            Expr::Var(sym) => {
                let name = interner.lookup(*sym);
                match locals[expr.0 as usize] {
                    Some(Local(local)) => _ = writeln!(out, "{pad}(var {name} :local {local})"),
                    None => _ = writeln!(out, "{pad}(var {name})"),
                }
            }
            Expr::Abs(param, body) => {
                let param = interner.lookup(*param);
                _ = writeln!(out, "{pad}(fun {param}");
                self.pretty_expr(*body, interner, locals, indent + 1, out);
                _ = writeln!(out, "{pad})");
            }
            Expr::App(fun, arg) => {
                _ = writeln!(out, "{pad}(app");
                self.pretty_expr(*fun, interner, locals, indent + 1, out);
                self.pretty_expr(*arg, interner, locals, indent + 1, out);
                _ = writeln!(out, "{pad})");
            }
            Expr::Bin(lhs, op, rhs) => {
                let _ = writeln!(out, "{pad}(bin {}", pretty_token(*op));
                self.pretty_expr(*lhs, interner, locals, indent + 1, out);
                self.pretty_expr(*rhs, interner, locals, indent + 1, out);
                let _ = writeln!(out, "{pad})");
            }
            Expr::Bind {
                is_recursive,
                name,
                init,
                body,
            } => {
                let kind = if *is_recursive { "let-rec" } else { "let" };
                let name = interner.lookup(*name);
                _ = writeln!(out, "{pad}({kind} {name}");
                self.pretty_expr(*init, interner, locals, indent + 1, out);
                self.pretty_expr(*body, interner, locals, indent + 1, out);
                _ = writeln!(out, "{pad})");
            }
            Expr::Cond {
                cond,
                then_branch,
                else_branch,
            } => {
                _ = writeln!(out, "{pad}(if");
                self.pretty_expr(*cond, interner, locals, indent + 1, out);
                self.pretty_expr(*then_branch, interner, locals, indent + 1, out);
                self.pretty_expr(*else_branch, interner, locals, indent + 1, out);
                _ = writeln!(out, "{pad})");
            }
        }
    }
}

fn pretty_token(token: Token) -> &'static str {
    match token {
        Token::Plus => "+",
        Token::Minus => "-",
        Token::Star => "*",
        Token::Slash => "/",
        Token::EqEq => "==",
        Token::BangEq => "!=",
        Token::Gt => ">",
        Token::GtEq => ">=",
        Token::Lt => "<",
        Token::LtEq => "<=",
        Token::And => "and",
        Token::Or => "or",
        _ => "?",
    }
}
