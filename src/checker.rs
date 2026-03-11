#![allow(unused)]

use std::collections::HashMap;

use crate::{
    arena::{Arena, ArenaIndex, Id},
    ast::{Ast, AstTable, Expr, ExprId, Lit},
    lexer::Token,
    resolver::Local,
};

type TypeId = Id<Type>;

pub struct Types {
    arena: Arena<Type>,
    int: TypeId,
    bool: TypeId,
    unit: TypeId,
}

impl Types {
    pub fn new() -> Self {
        let mut arena = Arena::new();
        let int = arena.alloc(Type::Int);
        let bool = arena.alloc(Type::Bool);
        let unit = arena.alloc(Type::Unit);

        Self {
            arena,
            int,
            bool,
            unit,
        }
    }

    #[inline]
    pub fn int(&self) -> TypeId {
        self.int
    }

    #[inline]
    pub fn bool(&self) -> TypeId {
        self.bool
    }

    #[inline]
    pub fn unit(&self) -> TypeId {
        self.unit
    }
}

impl std::ops::Deref for Types {
    type Target = Arena<Type>;

    fn deref(&self) -> &Self::Target {
        &self.arena
    }
}

impl std::ops::DerefMut for Types {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.arena
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct TypeVar(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Unit,
    Int,
    Bool,
    Var(TypeVar),
    Abs(TypeId, TypeId),
}

#[derive(Debug, Clone)]
struct Scheme {
    vars: Vec<TypeVar>,
    ty: TypeId,
}

type Env = Vec<Scheme>;
type Subst = HashMap<TypeVar, TypeId>;

#[derive(Debug, Clone)]
enum TypeError {
    Mismatch { left: TypeId, right: TypeId },
    InfiniteType { var: TypeVar, ty: TypeId },
}

pub struct Checker<'a> {
    ast: &'a Ast,
    tvar_count: u32,
    types: Types,
    locals: &'a AstTable<Option<Local>>,
    debug_indent: usize,
}

pub fn typecheck(ast: &Ast, expr: ExprId, locals: &AstTable<Option<Local>>) {
    let mut checker = Checker::new(ast, locals);
    match checker.infer_top(expr) {
        Ok(ty) => {
            println!("result => {}", checker.type_to_string(ty));
        }
        Err(error) => {
            println!("{}", checker.error_to_string(error));
        }
    }
}

impl<'a> Checker<'a> {
    pub fn new(ast: &'a Ast, locals: &'a AstTable<Option<Local>>) -> Self {
        Self {
            ast,
            tvar_count: 0,
            types: Types::new(),
            locals,
            debug_indent: 0,
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
        self.types.alloc(Type::Var(tvar))
    }

    fn mono(&self, ty: TypeId) -> Scheme {
        Scheme {
            vars: Vec::new(),
            ty,
        }
    }

    fn compose(&mut self, newer: &Subst, older: &Subst) -> Subst {
        let mut composed = Subst::with_capacity(newer.len() + older.len());
        for (&var, &ty) in older {
            composed.insert(var, self.apply(ty, newer));
        }
        for (&var, &ty) in newer {
            composed.insert(var, ty);
        }
        composed
    }

    fn apply_env(&mut self, env: &Env, subst: &Subst) -> Env {
        env.iter()
            .map(|scheme| self.apply_scheme(scheme, subst))
            .collect()
    }

    fn apply_scheme(&mut self, scheme: &Scheme, subst: &Subst) -> Scheme {
        let mut subst = subst.clone();
        for &var in &scheme.vars {
            subst.remove(&var);
        }
        Scheme {
            vars: scheme.vars.clone(),
            ty: self.apply(scheme.ty, &subst),
        }
    }

    fn apply(&mut self, ty_id: TypeId, subst: &Subst) -> TypeId {
        match self.types[ty_id] {
            Type::Unit | Type::Int | Type::Bool => ty_id,
            Type::Var(tvar) => match subst.get(&tvar).copied() {
                Some(ty) => self.apply(ty, subst),
                None => ty_id,
            },
            Type::Abs(param, body) => {
                let param_ty = self.apply(param, subst);
                let body_ty = self.apply(body, subst);
                self.types.alloc(Type::Abs(param_ty, body_ty))
            }
        }
    }

    fn free_type_vars(&self, ty: TypeId, acc: &mut Vec<TypeVar>) {
        match self.types[ty] {
            Type::Unit | Type::Int | Type::Bool => {}
            Type::Var(var) => {
                if !acc.contains(&var) {
                    acc.push(var);
                }
            }
            Type::Abs(param, body) => {
                self.free_type_vars(param, acc);
                self.free_type_vars(body, acc);
            }
        }
    }

    fn free_scheme_vars(&self, scheme: &Scheme, acc: &mut Vec<TypeVar>) {
        let mut vars = Vec::new();
        self.free_type_vars(scheme.ty, &mut vars);
        for var in vars {
            if !scheme.vars.contains(&var) && !acc.contains(&var) {
                acc.push(var);
            }
        }
    }

    fn free_env_vars(&self, env: &Env, acc: &mut Vec<TypeVar>) {
        for scheme in env {
            self.free_scheme_vars(scheme, acc);
        }
    }

    fn instantiate(&mut self, scheme: &Scheme) -> TypeId {
        let mut subst = Subst::new();
        for &var in &scheme.vars {
            subst.insert(var, self.fresh_tvar());
        }
        self.apply(scheme.ty, &subst)
    }

    fn generalize(&mut self, env: &Env, ty: TypeId, subst: &Subst) -> Scheme {
        let ty = self.apply(ty, subst);
        let env = self.apply_env(env, subst);

        let mut ty_vars = Vec::new();
        self.free_type_vars(ty, &mut ty_vars);

        let mut env_vars = Vec::new();
        self.free_env_vars(&env, &mut env_vars);

        let vars = ty_vars
            .into_iter()
            .filter(|var| !env_vars.contains(var))
            .collect();

        Scheme { vars, ty }
    }

    fn occurs_in(&mut self, needle: TypeVar, ty: TypeId, subst: &Subst) -> bool {
        let ty = self.apply(ty, subst);
        match self.types[ty] {
            Type::Unit | Type::Int | Type::Bool => false,
            Type::Var(var) => var == needle,
            Type::Abs(param, body) => {
                self.occurs_in(needle, param, subst) || self.occurs_in(needle, body, subst)
            }
        }
    }

    fn bind(&mut self, var: TypeVar, ty: TypeId, subst: &mut Subst) -> Result<(), TypeError> {
        let ty = self.apply(ty, subst);
        if matches!(self.types[ty], Type::Var(found) if found == var) {
            return Ok(());
        }
        if self.occurs_in(var, ty, subst) {
            return Err(TypeError::InfiniteType { var, ty });
        }
        self.debug(format_args!(
            "bind {} := {}",
            self.type_var_to_string(var),
            self.type_to_string(ty)
        ));
        subst.insert(var, ty);
        Ok(())
    }

    fn unify(&mut self, left: TypeId, right: TypeId, subst: &mut Subst) -> Result<(), TypeError> {
        let left = self.apply(left, subst);
        let right = self.apply(right, subst);

        self.debug(format_args!(
            "unify {} ~ {}",
            self.type_to_string(left),
            self.type_to_string(right)
        ));

        match (self.types[left], self.types[right]) {
            (Type::Unit, Type::Unit) | (Type::Int, Type::Int) | (Type::Bool, Type::Bool) => Ok(()),
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
        self.debug_enter(format_args!("infer {}", self.expr_label(expr)));
        let result = self.infer_impl(expr, env);
        match &result {
            Ok((subst, ty)) => {
                let ty = self.apply(*ty, subst);
                let ty = self.type_to_string(ty);
                let subst = self.subst_to_string(subst);
                self.debug(format_args!("=> {} with {}", ty, subst));
            }
            Err(err) => {
                let err = self.error_to_string(err.clone());
                self.debug(format_args!("=> error: {}", err));
            }
        }
        self.debug_exit();
        result
    }

    fn infer_impl(&mut self, expr: ExprId, env: &Env) -> Result<(Subst, TypeId), TypeError> {
        match &self.ast[expr] {
            // T-Lit
            // Γ ⊢ n : int
            // Γ ⊢ true : bool
            // Γ ⊢ () : unit
            Expr::Lit(lit) => Ok((
                Subst::new(),
                match lit {
                    Lit::Unit => self.types.unit,
                    Lit::Int(_) => self.types.int,
                    Lit::Bool(_) => self.types.bool,
                },
            )),

            // T-Var
            // x : σ ∈ Γ
            // ------------------------
            // Γ ⊢ x : instantiate(σ)
            Expr::Var(_) => {
                let local = self.locals[expr].unwrap();
                let scheme = &env[local.0 as usize];
                let ty = self.instantiate(scheme);
                self.debug(format_args!(
                    "instantiate {} as {}",
                    self.scheme_to_string(scheme),
                    self.type_to_string(ty)
                ));
                Ok((Subst::new(), ty))
            }

            // T-Abs
            // Γ, x : α ⊢ e : τ
            // -----------------------
            // Γ ⊢ \x.e : α -> τ
            Expr::Abs(_, body) => {
                let param_ty = self.fresh_tvar();
                let mut body_env = env.clone();
                body_env.push(self.mono(param_ty));

                let (subst, body_ty) = self.infer(*body, &body_env)?;
                let param_ty = self.apply(param_ty, &subst);
                let fun_ty = self.types.alloc(Type::Abs(param_ty, body_ty));

                Ok((subst, fun_ty))
            }

            // T-App
            // Γ ⊢ f : τ1      Γ ⊢ a : τ2      β fresh      τ1 ~ τ2 -> β
            // ---------------------------------------------------------
            // Γ ⊢ f a : β
            Expr::App(lhs, arg) => {
                let (subst_fun, fun_ty) = self.infer(*lhs, env)?;
                let arg_env = self.apply_env(env, &subst_fun);
                let (subst_arg, arg_ty) = self.infer(*arg, &arg_env)?;

                let mut subst = self.compose(&subst_arg, &subst_fun);
                let result_ty = self.fresh_tvar();
                let arg_ty = self.apply(arg_ty, &subst);
                let expected_fun_ty = self.types.alloc(Type::Abs(arg_ty, result_ty));

                self.unify(fun_ty, expected_fun_ty, &mut subst)?;

                Ok((subst.clone(), self.apply(result_ty, &subst)))
            }

            // Primitive operators
            // These are checked by assigning each operator a fixed type schema.
            Expr::Bin(lhs, op, rhs) => {
                let (subst_lhs, lhs_ty) = self.infer(*lhs, env)?;
                let rhs_env = self.apply_env(env, &subst_lhs);
                let (subst_rhs, rhs_ty) = self.infer(*rhs, &rhs_env)?;

                let mut subst = self.compose(&subst_rhs, &subst_lhs);
                let result_ty = match op {
                    Token::Plus | Token::Minus | Token::Star | Token::Slash => {
                        // let int_ty = self.arena.alloc(Type::Int);
                        self.unify(lhs_ty, self.types.int, &mut subst)?;
                        self.unify(rhs_ty, self.types.int, &mut subst)?;
                        self.types.int
                        // int_ty
                    }
                    Token::Gt | Token::GtEq | Token::Lt | Token::LtEq => {
                        // let int_ty = self.arena.alloc(Type::Int);
                        self.unify(lhs_ty, self.types.int, &mut subst)?;
                        self.unify(rhs_ty, self.types.int, &mut subst)?;
                        // self.arena.alloc(Type::Bool)
                        self.types.int
                    }
                    Token::EqEq | Token::BangEq => {
                        self.unify(lhs_ty, rhs_ty, &mut subst)?;
                        // self.arena.alloc(Type::Bool)
                        self.types.int
                    }
                    _ => unreachable!("parser only builds binary expressions for binary operators"),
                };

                Ok((subst.clone(), self.apply(result_ty, &subst)))
            }

            // T-Let / T-LetRec
            // Γ ⊢ e1 : τ1      σ = generalize(Γ, τ1)      Γ, x : σ ⊢ e2 : τ2
            // ----------------------------------------------------------------
            // Γ ⊢ let x = e1 in e2 : τ2
            Expr::Bind {
                is_recursive,
                name: _,
                init,
                body,
            } => {
                if *is_recursive {
                    let placeholder_ty = self.fresh_tvar();
                    let mut init_env = env.clone();
                    init_env.push(self.mono(placeholder_ty));

                    let (subst_init, init_ty) = self.infer(*init, &init_env)?;
                    let mut subst = subst_init;
                    self.unify(placeholder_ty, init_ty, &mut subst)?;

                    let mut body_env = self.apply_env(env, &subst);
                    let scheme = self.generalize(env, placeholder_ty, &subst);
                    self.debug(format_args!(
                        "generalize rec binding as {}",
                        self.scheme_to_string(&scheme)
                    ));
                    body_env.push(scheme);

                    let (subst_body, body_ty) = self.infer(*body, &body_env)?;
                    let subst = self.compose(&subst_body, &subst);

                    Ok((subst.clone(), self.apply(body_ty, &subst)))
                } else {
                    let (subst_init, init_ty) = self.infer(*init, env)?;
                    let mut body_env = self.apply_env(env, &subst_init);
                    let scheme = self.generalize(env, init_ty, &subst_init);
                    self.debug(format_args!(
                        "generalize binding as {}",
                        self.scheme_to_string(&scheme)
                    ));
                    body_env.push(scheme);

                    let (subst_body, body_ty) = self.infer(*body, &body_env)?;
                    let subst = self.compose(&subst_body, &subst_init);

                    Ok((subst.clone(), self.apply(body_ty, &subst)))
                }
            }

            // T-If
            // Γ ⊢ c : bool      Γ ⊢ t : τ      Γ ⊢ e : τ
            // ------------------------------------------
            // Γ ⊢ if c then t else e : τ
            Expr::Cond {
                cond,
                then_branch,
                else_branch,
            } => {
                let (subst_cond, cond_ty) = self.infer(*cond, env)?;
                let mut subst = subst_cond;
                // let bool_ty = self.arena.alloc(Type::Bool);
                self.unify(cond_ty, self.types.bool, &mut subst)?;

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

    fn debug(&self, args: std::fmt::Arguments<'_>) {
        let pad = "  ".repeat(self.debug_indent);
        eprintln!("{pad}{args}");
    }

    fn debug_enter(&mut self, args: std::fmt::Arguments<'_>) {
        self.debug(args);
        self.debug_indent += 1;
    }

    fn debug_exit(&mut self) {
        self.debug_indent = self.debug_indent.saturating_sub(1);
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

    fn scheme_to_string(&self, scheme: &Scheme) -> String {
        if scheme.vars.is_empty() {
            return self.type_to_string(scheme.ty);
        }

        let vars = scheme
            .vars
            .iter()
            .map(|var| self.type_var_to_string(*var))
            .collect::<Vec<_>>()
            .join(" ");
        format!("forall {vars}. {}", self.type_to_string(scheme.ty))
    }

    fn subst_to_string(&mut self, subst: &Subst) -> String {
        if subst.is_empty() {
            return "{}".to_string();
        }

        let mut entries = subst
            .iter()
            .map(|(var, ty)| {
                let ty = self.apply(*ty, subst);
                let ty = self.type_to_string(ty);
                format!("{} := {}", self.type_var_to_string(*var), ty)
            })
            .collect::<Vec<_>>();
        entries.sort();
        format!("{{{}}}", entries.join(", "))
    }

    fn expr_label(&self, expr: ExprId) -> String {
        match &self.ast[expr] {
            Expr::Lit(Lit::Unit) => "()".to_string(),
            Expr::Lit(Lit::Int(n)) => n.to_string(),
            Expr::Lit(Lit::Bool(b)) => b.to_string(),
            Expr::Var(_) => format!("var@{}", expr.index()),
            Expr::Abs(_, _) => format!("fun@{}", expr.index()),
            Expr::App(_, _) => format!("app@{}", expr.index()),
            Expr::Bin(_, op, _) => format!("bin({op:?})@{}", expr.index()),
            Expr::Bind { is_recursive, .. } => {
                if *is_recursive {
                    format!("let-rec@{}", expr.index())
                } else {
                    format!("let@{}", expr.index())
                }
            }
            Expr::Cond { .. } => format!("if@{}", expr.index()),
        }
    }

    fn type_to_string(&self, id: TypeId) -> String {
        match self.types[id] {
            Type::Unit => "()".to_string(),
            Type::Int => "int".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Var(var) => self.type_var_to_string(var),
            Type::Abs(param, ret) => {
                let param = match self.types[param] {
                    Type::Abs(_, _) => format!("({})", self.type_to_string(param)),
                    _ => self.type_to_string(param),
                };
                format!("{param} -> {}", self.type_to_string(ret))
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
    fn polymorphic_let_allows_multiple_instantiations() {
        let ty = infer_source("let id = \\x.x in let a = id 1 in id true").unwrap();
        assert_eq!(ty, "bool");
    }

    #[test]
    fn rejects_infinite_types() {
        let err = infer_source("\\x. x x").unwrap_err();
        assert!(err.contains("illegal recursive type"), "{err}");
    }
}
