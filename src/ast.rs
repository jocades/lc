use crate::Symbol;
use crate::lexer::{Span, Token};

#[derive(Debug, Clone, Copy)]
pub struct ExprId(u32);

impl ExprId {
    #[inline]
    pub fn get(&self) -> u32 {
        self.0
    }
}

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

pub enum Foo {
    One(i32),
    Two,
    Three,
}

#[derive(Debug)]
pub enum Expr {
    Lit(Lit),
    Var(Symbol),
    Fun(Symbol, ExprId),
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
