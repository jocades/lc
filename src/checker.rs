#![allow(unused)]

use std::collections::HashMap;

use crate::{
    ast::{Ast, Expr, ExprId, Lit},
    lexer::Token,
    resolver::Local,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeId(u32);

#[derive(Default)]
struct Arena {
    types: Vec<Type>,
}

impl Arena {
    fn alloc(&mut self, ty: Type) -> TypeId {
        let id = TypeId(self.types.len() as u32);
        self.types.push(ty);
        id
    }

    fn get(&self, id: TypeId) -> &Type {
        &self.types[id.0 as usize]
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct TypeVar(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Type {
    Unit,
    Int,
    Bool,
    Var(TypeVar),
    Abs(TypeId, TypeId),
}

/// Mapping of `Local` to inferred monomorphic types.
type Env = Vec<TypeId>;

type Subst = HashMap<TypeVar, TypeId>;

#[derive(Debug)]
enum TypeError {
    Mismatch { left: TypeId, right: TypeId },
    InfiniteType { var: TypeVar, ty: TypeId },
}

pub struct Checker<'a> {
    ast: &'a Ast,
    tvar_count: u32,
    arena: Arena,
    locals: &'a [Option<Local>],
}

pub fn typecheck(ast: &Ast, expr: ExprId, locals: &[Option<Local>]) {
    let mut checker = Checker::new(ast, locals);
    match checker.infer_top(expr) {
        Ok(ty) => {
            println!("result => {}", checker.type_to_string(ty));
        }
        Err(error) => {
            panic!("{}", checker.error_to_string(error));
        }
    }
}

impl<'a> Checker<'a> {
    pub fn new(ast: &'a Ast, locals: &'a [Option<Local>]) -> Self {
        Self {
            ast,
            tvar_count: 0,
            arena: Arena::default(),
            locals,
        }
    }

    fn infer_top(&mut self, expr: ExprId) -> Result<TypeId, TypeError> {
        let env = Env::new();
        let (subst, ty) = self.infer(expr, &env)?;
        Ok(self.apply(ty, &subst))
    }

    fn fresh_tvar(&mut self) -> TypeId {
        let tvar = TypeVar(self.tvar_count);
        self.tvar_count += 1;
        self.arena.alloc(Type::Var(tvar))
    }

    fn compose(&mut self, newer: &Subst, older: &Subst) -> Subst {
        let mut composed = HashMap::with_capacity(newer.len() + older.len());
        for (&var, &ty) in older {
            composed.insert(var, self.apply(ty, newer));
        }
        for (&var, &ty) in newer {
            composed.insert(var, ty);
        }
        composed
    }

    fn apply_env(&mut self, env: &Env, subst: &Subst) -> Env {
        env.iter().map(|&ty| self.apply(ty, subst)).collect()
    }

    fn apply(&mut self, ty_id: TypeId, subst: &Subst) -> TypeId {
        match *self.arena.get(ty_id) {
            Type::Unit | Type::Int | Type::Bool => ty_id,
            Type::Var(tvar) => match subst.get(&tvar).copied() {
                Some(ty) => self.apply(ty, subst),
                None => ty_id,
            },
            Type::Abs(param, body) => {
                let param_ty = self.apply(param, subst);
                let body_ty = self.apply(body, subst);
                self.arena.alloc(Type::Abs(param_ty, body_ty))
            }
        }
    }

    fn occurs_in(&mut self, needle: TypeVar, ty: TypeId, subst: &Subst) -> bool {
        let ty = self.apply(ty, subst);
        match *self.arena.get(ty) {
            Type::Unit | Type::Int | Type::Bool => false,
            Type::Var(var) => var == needle,
            Type::Abs(param, body) => {
                self.occurs_in(needle, param, subst) || self.occurs_in(needle, body, subst)
            }
        }
    }

    fn bind(&mut self, var: TypeVar, ty: TypeId, subst: &mut Subst) -> Result<(), TypeError> {
        let ty = self.apply(ty, subst);
        if matches!(self.arena.get(ty), Type::Var(found) if *found == var) {
            return Ok(());
        }
        if self.occurs_in(var, ty, subst) {
            return Err(TypeError::InfiniteType { var, ty });
        }
        subst.insert(var, ty);
        Ok(())
    }

    fn unify(&mut self, left: TypeId, right: TypeId, subst: &mut Subst) -> Result<(), TypeError> {
        let left = self.apply(left, subst);
        let right = self.apply(right, subst);

        match (*self.arena.get(left), *self.arena.get(right)) {
            (Type::Unit, Type::Unit) | (Type::Int, Type::Int) | (Type::Bool, Type::Bool) => {
                Ok(())
            }
            (Type::Var(a), Type::Var(b)) if a == b => Ok(()),
            (Type::Var(var), _) => self.bind(var, right, subst),
            (_, Type::Var(var)) => self.bind(var, left, subst),
            (Type::Abs(param_a, body_a), Type::Abs(param_b, body_b)) => {
                self.unify(param_a, param_b, subst)?;
                self.unify(body_a, body_b, subst)
            }
            _ => Err(TypeError::Mismatch { left, right }),
        }
    }

    fn infer(&mut self, expr: ExprId, env: &Env) -> Result<(Subst, TypeId), TypeError> {
        match &self.ast[expr] {
            Expr::Lit(lit) => Ok((
                Subst::new(),
                match lit {
                    Lit::Unit => self.arena.alloc(Type::Unit),
                    Lit::Int(_) => self.arena.alloc(Type::Int),
                    Lit::Bool(_) => self.arena.alloc(Type::Bool),
                },
            )),
            Expr::Var(_) => {
                let local = self.locals[expr.0 as usize].unwrap();
                Ok((Subst::new(), env[local.0 as usize]))
            }
            Expr::Fun(_, body) => {
                let param_ty = self.fresh_tvar();
                let mut body_env = env.clone();
                body_env.push(param_ty);

                let (subst, body_ty) = self.infer(*body, &body_env)?;
                let param_ty = self.apply(param_ty, &subst);
                let fun_ty = self.arena.alloc(Type::Abs(param_ty, body_ty));

                Ok((subst, fun_ty))
            }
            Expr::App(lhs, arg) => {
                let (subst_fun, fun_ty) = self.infer(*lhs, env)?;
                let arg_env = self.apply_env(env, &subst_fun);
                let (subst_arg, arg_ty) = self.infer(*arg, &arg_env)?;

                let mut subst = self.compose(&subst_arg, &subst_fun);
                let result_ty = self.fresh_tvar();
                let arg_ty = self.apply(arg_ty, &subst);
                let expected_fun_ty = self.arena.alloc(Type::Abs(arg_ty, result_ty));

                self.unify(fun_ty, expected_fun_ty, &mut subst)?;

                Ok((subst.clone(), self.apply(result_ty, &subst)))
            }
            Expr::Bin(lhs, op, rhs) => {
                let (subst_lhs, lhs_ty) = self.infer(*lhs, env)?;
                let rhs_env = self.apply_env(env, &subst_lhs);
                let (subst_rhs, rhs_ty) = self.infer(*rhs, &rhs_env)?;

                let mut subst = self.compose(&subst_rhs, &subst_lhs);
                let result_ty = match op {
                    Token::Plus | Token::Minus | Token::Star | Token::Slash => {
                        let int_ty = self.arena.alloc(Type::Int);
                        self.unify(lhs_ty, int_ty, &mut subst)?;
                        self.unify(rhs_ty, int_ty, &mut subst)?;
                        int_ty
                    }
                    Token::Gt | Token::GtEq | Token::Lt | Token::LtEq => {
                        let int_ty = self.arena.alloc(Type::Int);
                        self.unify(lhs_ty, int_ty, &mut subst)?;
                        self.unify(rhs_ty, int_ty, &mut subst)?;
                        self.arena.alloc(Type::Bool)
                    }
                    Token::EqEq | Token::BangEq => {
                        self.unify(lhs_ty, rhs_ty, &mut subst)?;
                        self.arena.alloc(Type::Bool)
                    }
                    _ => unreachable!("parser only builds binary expressions for binary operators"),
                };

                Ok((subst.clone(), self.apply(result_ty, &subst)))
            }
            Expr::Bind {
                is_recursive,
                name: _,
                init,
                body,
            } => {
                if *is_recursive {
                    let placeholder_ty = self.fresh_tvar();
                    let mut init_env = env.clone();
                    init_env.push(placeholder_ty);

                    let (subst_init, init_ty) = self.infer(*init, &init_env)?;
                    let mut subst = subst_init;
                    self.unify(placeholder_ty, init_ty, &mut subst)?;

                    let mut body_env = self.apply_env(env, &subst);
                    body_env.push(self.apply(placeholder_ty, &subst));

                    let (subst_body, body_ty) = self.infer(*body, &body_env)?;
                    let subst = self.compose(&subst_body, &subst);

                    Ok((subst.clone(), self.apply(body_ty, &subst)))
                } else {
                    let (subst_init, init_ty) = self.infer(*init, env)?;
                    let mut body_env = self.apply_env(env, &subst_init);
                    body_env.push(self.apply(init_ty, &subst_init));

                    let (subst_body, body_ty) = self.infer(*body, &body_env)?;
                    let subst = self.compose(&subst_body, &subst_init);

                    Ok((subst.clone(), self.apply(body_ty, &subst)))
                }
            }
            Expr::Cond {
                cond,
                then_branch,
                else_branch,
            } => {
                let (subst_cond, cond_ty) = self.infer(*cond, env)?;
                let mut subst = subst_cond;
                let bool_ty = self.arena.alloc(Type::Bool);
                self.unify(cond_ty, bool_ty, &mut subst)?;

                let then_env = self.apply_env(env, &subst);
                let (subst_then, then_ty) = self.infer(*then_branch, &then_env)?;
                subst = self.compose(&subst_then, &subst);

                let else_env = self.apply_env(env, &subst);
                let (subst_else, else_ty) = self.infer(*else_branch, &else_env)?;
                subst = self.compose(&subst_else, &subst);

                self.unify(then_ty, else_ty, &mut subst)?;

                Ok((subst.clone(), self.apply(then_ty, &subst)))
            }
        }
    }

    fn error_to_string(&mut self, error: TypeError) -> String {
        match error {
            TypeError::Mismatch { left, right } => {
                format!(
                    "type mismatch: {} != {}",
                    self.type_to_string(left),
                    self.type_to_string(right)
                )
            }
            TypeError::InfiniteType { var, ty } => format!(
                "illegal recursive type: {} occurs in {}",
                self.type_var_to_string(var),
                self.type_to_string(ty)
            ),
        }
    }

    fn type_var_to_string(&self, var: TypeVar) -> String {
        format!("t{}", var.0)
    }

    fn type_to_string(&self, id: TypeId) -> String {
        match self.arena.get(id) {
            Type::Unit => "()".to_string(),
            Type::Int => "int".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Var(var) => self.type_var_to_string(*var),
            Type::Abs(param, ret) => {
                let param = match self.arena.get(*param) {
                    Type::Abs(_, _) => format!("({})", self.type_to_string(*param)),
                    _ => self.type_to_string(*param),
                };
                format!("{param} -> {}", self.type_to_string(*ret))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{interner::Interner, parser, resolver};

    fn infer_source(source: &str) -> Result<String, String> {
        let mut interner = Interner::with_capacity(64);
        let (ast, expr) = parser::parse(source, &mut interner).unwrap();
        let locals = resolver::resolve(&ast, expr, &interner);
        let mut checker = Checker::new(&ast, &locals);
        checker
            .infer_top(expr)
            .map(|ty| checker.type_to_string(ty))
            .map_err(|err| checker.error_to_string(err))
    }

    #[test]
    fn infers_identity_application() {
        let ty = infer_source("let id = \\x.x in id 1").unwrap();
        assert_eq!(ty, "int");
    }

    #[test]
    fn keeps_let_monomorphic() {
        let err = infer_source("let id = \\x.x in let a = id 1 in id true").unwrap_err();
        assert!(err.contains("int != bool"), "{err}");
    }

    #[test]
    fn rejects_infinite_types() {
        let err = infer_source("\\x. x x").unwrap_err();
        assert!(err.contains("illegal recursive type"), "{err}");
    }
}
