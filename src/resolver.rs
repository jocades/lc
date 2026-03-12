use std::collections::HashMap;

use crate::Symbol;
use crate::ast::{Ast, AstTable, Expr, ExprId, Table};
use crate::interner::Interner;

pub struct Resolution {
    pub uses: AstTable<Option<Local>>,
    pub binders: AstTable<Option<Local>>,
}

pub fn resolve(ast: &Ast, root: ExprId, interner: &Interner) -> Resolution {
    let mut resolver = Resolver::new(ast, interner);
    resolver.resolve(root);
    Resolution {
        uses: resolver.uses,
        binders: resolver.binders,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Local(pub u32);

#[derive(Debug, PartialEq)]
enum VarState {
    Declared,
    // Defined,
    Read,
}

#[derive(Debug)]
struct Var {
    local: Local,
    state: VarState,
}

impl Var {
    fn new(local: Local, state: VarState) -> Self {
        Self { local, state }
    }
}

type Scope = HashMap<Symbol, Var>;

struct Resolver<'a> {
    /// A pointer to the `Ast` to retrieve `Expr`s.
    ast: &'a Ast,
    /// A pointer to the string `interner` to print diagnostics.
    interner: &'a Interner,
    /// Stack of variable scopes
    scopes: Vec<Scope>,
    /// Array of locals parallel to `Ast::nodes`.
    uses: Table<ExprId, Option<Local>>,
    binders: Table<ExprId, Option<Local>>,
    /// Monotonic counter for globally unique binding IDs.
    next_local: u32,
}

impl<'a> Resolver<'a> {
    fn new(ast: &'a Ast, interner: &'a Interner) -> Self {
        Self {
            ast,
            interner,
            scopes: vec![Scope::new()],
            uses: ast.table(None),
            binders: ast.table(None),
            next_local: 0,
        }
    }

    fn scope(&mut self) -> &mut Scope {
        self.scopes
            .last_mut()
            .expect("there should always be at least one scope")
    }

    fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    fn exit_scope(&mut self) {
        let scope = self.scopes.pop().unwrap();
        for (sym, var) in scope {
            if var.state != VarState::Read {
                println!(
                    "Local variable `{}` is defined but never used.",
                    self.interner.lookup(sym)
                )
            }
        }
    }

    fn with_scope(&mut self, f: impl Fn(&mut Self)) {
        self.enter_scope();
        f(self);
        self.exit_scope();
    }

    fn declare(&mut self, sym: Symbol) -> Local {
        let local = Local(self.next_local);
        self.next_local += 1;
        self.scope()
            .insert(sym, Var::new(local, VarState::Declared));
        local
    }

    fn lookup(&mut self, sym: Symbol) -> Option<Local> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(&sym) {
                var.state = VarState::Read;
                return Some(var.local);
            }
        }
        panic!("unbound variable `{}`", self.interner.lookup(sym));
    }

    fn resolve(&mut self, expr: ExprId) {
        match &self.ast[expr] {
            Expr::Lit(_) => {}
            Expr::Var(sym) => {
                let local = self.lookup(*sym);
                self.uses[expr] = local;
            }
            Expr::Abs(param, body) => {
                self.with_scope(|this| {
                    let binder = this.declare(*param);
                    this.binders[expr] = Some(binder);
                    this.resolve(*body);
                });
            }
            Expr::App(fun, arg) => {
                self.resolve(*fun);
                self.resolve(*arg);
            }
            Expr::Bin(lhs, _, rhs) => {
                self.resolve(*lhs);
                self.resolve(*rhs);
            }
            Expr::Bind {
                is_recursive,
                name,
                init,
                body,
            } => {
                if !is_recursive {
                    self.resolve(*init);
                    self.with_scope(|this| {
                        let binder = this.declare(*name);
                        this.binders[expr] = Some(binder);
                        this.resolve(*body);
                    });
                } else {
                    self.with_scope(|this| {
                        let binder = this.declare(*name);
                        this.binders[expr] = Some(binder);
                        this.resolve(*init);
                        this.resolve(*body);
                    });
                }
            }
            Expr::Cond {
                cond,
                then_branch,
                else_branch,
            } => {
                self.resolve(*cond);
                self.resolve(*then_branch);
                self.resolve(*else_branch);
            }
            Expr::Error => unreachable!(),
        }
    }
}
