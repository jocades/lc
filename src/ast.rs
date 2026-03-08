use crate::Symbol;
use crate::lexer::{Span, Token};

#[derive(Debug, Clone, Copy)]
pub struct ExprId(u32);

#[derive(Default)]
pub struct Ast {
    nodes: Vec<Expr>,
    // spans: Vec<Span>,
}

impl Ast {
    pub fn alloc(&mut self, expr: Expr) -> ExprId {
        let id = ExprId(self.nodes.len() as u32);
        self.nodes.push(expr);
        // self.spans.push(span);
        id
    }

    pub fn get(&self, id: ExprId) -> &Expr {
        &self.nodes[id.0 as usize]
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
    Lit(Lit, Span),
    Var(Symbol, Span),
    Abs(Symbol, ExprId, Span),
    App(ExprId, ExprId, Span),
    Bin(ExprId, Token, ExprId, Span),
    Bind {
        is_recursive: bool,
        name: Symbol,
        init: ExprId,
        body: ExprId,
        span: Span,
    },
    Cond {
        cond: ExprId,
        then_branch: ExprId,
        else_branch: ExprId,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
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
