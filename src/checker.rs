/* use std::collections::HashMap;

use crate::Symbol;
use crate::ast::{Ast, Expr, ExprId, Lit};

#[derive(Debug, Clone, Copy)]
struct TypeId(u32);

type Env = HashMap<Symbol, Type>;

#[derive(Default)]
struct Context {
    types: Vec<Type>,
}

impl Context {
    fn alloc(&mut self, ty: Type) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(ty);
        id
    }

    fn get(&self, id: TypeId) -> &Type {
        &self.types[id.0 as usize]
    }
}

struct TypedSymbol(Symbol, Type);

#[derive(Debug, Clone, Copy)]
struct TypeVar(u32);

#[derive(Debug, Clone, Copy)]
enum Type {
    Int,
    Var(TypeVar),
    Abs(TypeId, TypeId),
}

enum Constraint {
    TypeEqual(ExprId, Type, Type),
}

struct GenOut {
    // Set of constraints to be solved
    constraints: Vec<Constraint>,
    // Expr where all variables are annotated with their type
    typed_expr: Expr<TypedSymbol>,
}

impl GenOut {
    fn new(constraints: Vec<Constraint>, typed_expr: Expr<TypedSymbol>) -> Self {
        Self {
            constraints,
            typed_expr,
        }
    }
}

fn infer(cx: &mut Context, env: &mut Env, ast: &Ast, expr: ExprId) -> (GenOut, Type) {
    match &ast[expr] {
        Expr::Lit(lit) => match lit {
            Lit::Unit => todo!(),
            Lit::Int(n) => (GenOut::new(vec![], Expr::Lit(Lit::Int(*n))), Type::Int),
            Lit::Bool(_) => todo!(),
        },
        Expr::Var(sym) => {
            let ty = *env.get(sym).unwrap();
            (GenOut::new(vec![], Expr::Var(TypedSymbol(*sym, ty))), ty)
        }
        Expr::Abs(_, expr_id) => todo!(),
        Expr::App(expr_id, expr_id1) => todo!(),
        Expr::Bin(expr_id, token, expr_id1) => todo!(),
        Expr::Bind {
            is_recursive,
            name,
            init,
            body,
        } => todo!(),
        Expr::Cond {
            cond,
            then_branch,
            else_branch,
        } => todo!(),
    }
}

fn check(cx: &mut Context, env: &mut Env, ty: Type) {} */
