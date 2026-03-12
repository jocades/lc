#![allow(unused)]

use std::collections::HashMap;

use crate::{
    ast::{Ast, AstTable, Expr, ExprId, Lit},
    checker::TypeId,
    interner::Symbol,
    lexer::Token,
    resolver::{Local, Resolution},
};

pub type FunId = u32;
pub type BlockId = u32;
pub type TempId = u32;
pub type LocalSlot = u32;
pub type EnvSlot = u32;

#[derive(Default, Debug)]
pub struct Program {
    pub funs: Vec<Fun>,
    pub entry: FunId,
}

#[derive(Debug)]
pub struct Fun {
    pub id: FunId,
    pub name: Option<Symbol>,
    pub arity: u16,
    pub captures: Vec<Capture>,
    pub locals: u32,
    pub temps: u32,
    pub blocks: Vec<Block>,
}

#[derive(Debug)]
pub struct Capture {
    pub source: CaptureSource,
}

#[derive(Debug)]
pub enum CaptureSource {
    Local(LocalSlot),
    Env(EnvSlot),
}

#[derive(Debug)]
pub struct Block {
    pub id: BlockId,
    pub instrs: Vec<Instr>,
    pub term: Terminator,
}

#[derive(Debug)]
pub enum Value {
    Temp(TempId),
    Local(LocalSlot),
    Env(EnvSlot),
    Int(i32),
    Bool(bool),
    Unit,
    Fun(FunId),
}

#[derive(Debug)]
pub enum Instr {
    LoadConst {
        dst: TempId,
        value: Value,
    },
    Move {
        dst: TempId,
        src: Value,
    },
    StoreLocal {
        dst: LocalSlot,
        src: Value,
    },
    BinOp {
        dst: TempId,
        op: BinOp,
        lhs: Value,
        rhs: Value,
    },
    MakeClosure {
        dst: TempId,
        fun: FunId,
        captures: Vec<Value>,
    },
    Call {
        dst: TempId,
        callee: Value,
        args: Vec<Value>,
    },
}

#[derive(Debug)]
pub enum Terminator {
    Unset,
    Jump(BlockId),
    Branch {
        cond: Value,
        then_block: BlockId,
        else_block: BlockId,
    },
    Return(Value),
}

#[derive(Debug)]
pub enum BinOp {
    AddInt,
    SubInt,
    MulInt,
    DivInt,
    EqInt,
    EqBool,
    LtInt,
    LeInt,
    GtInt,
    GeInt,
}

#[derive(Default)]
struct FunContext {
    fun: FunId,
    current_block: BlockId,
    next_temp: TempId,
    next_local: LocalSlot,

    resolved_locals: HashMap<Local, LocalSlot>,
    captures: HashMap<Local, EnvSlot>,
}

impl FunContext {
    fn fresh_temp(&mut self) -> TempId {
        let temp = self.next_temp;
        self.next_temp += 1;
        temp
    }

    fn fresh_local(&mut self) -> LocalSlot {
        let local = self.next_local;
        self.next_local += 1;
        local
    }
}

pub struct Lowerer<'a> {
    pub ast: &'a Ast,
    pub resolution: &'a Resolution,
    pub types: &'a AstTable<Option<TypeId>>,
    pub program: Program,
}

// Lowering invariants:
//
// - `lower_expr` appends instructions to the current block and returns the IR
//   value where the expression result lives.
// - Non-control-flow expressions do not create or terminate blocks.
// - Control-flow expressions such as `if` may create blocks and install
//   terminators, but still return a `Value` that represents the expression
//   result to later code.
// - Resolver `Local`s identify source bindings; lowering maps them to runtime
//   `LocalSlot`s in the current frame or, later, to `EnvSlot`s for captures.
impl<'a> Lowerer<'a> {
    pub fn new(
        ast: &'a Ast,
        resolution: &'a Resolution,
        types: &'a AstTable<Option<TypeId>>,
    ) -> Self {
        let program = Program {
            funs: Vec::new(),
            entry: 0,
        };

        Self {
            ast,
            resolution,
            types,
            program,
        }
    }

    pub fn lower(&mut self, expr: ExprId) {
        let fun_id = 0;
        let block_id = 0;

        {
            self.program.entry = fun_id;

            let main_fun = Fun {
                id: fun_id,
                name: None,
                arity: 0,
                captures: vec![],
                locals: 0,
                temps: 0,
                blocks: vec![Block {
                    id: block_id,
                    instrs: vec![],
                    term: Terminator::Unset,
                }],
            };

            self.program.funs.push(main_fun);
        }

        let mut cx = FunContext {
            fun: fun_id,
            current_block: block_id,
            next_temp: 0,
            next_local: 0,
            resolved_locals: HashMap::new(),
            captures: HashMap::new(),
        };

        let result = self.lower_expr(expr, &mut cx);

        let fun = &mut self.program.funs[cx.fun as usize];
        fun.locals = cx.next_local;
        fun.temps = cx.next_temp;
        fun.blocks[cx.current_block as usize].term = Terminator::Return(result);

        dbg!(&self.program);
    }

    fn current_fun_mut(&mut self, cx: &FunContext) -> &mut Fun {
        &mut self.program.funs[cx.fun as usize]
    }

    fn current_block_mut(&mut self, cx: &FunContext) -> &mut Block {
        &mut self.current_fun_mut(cx).blocks[cx.current_block as usize]
    }

    fn push_instr(&mut self, cx: &FunContext, instr: Instr) {
        self.current_block_mut(cx).instrs.push(instr);
    }

    fn set_terminator(&mut self, cx: &FunContext, term: Terminator) {
        self.current_block_mut(cx).term = term;
    }

    fn fresh_block(&mut self, cx: &FunContext) -> BlockId {
        let fun = self.current_fun_mut(cx);
        let id = fun.blocks.len() as BlockId;
        fun.blocks.push(Block {
            id,
            instrs: vec![],
            term: Terminator::Unset,
        });
        id
    }

    fn lower_expr(&mut self, expr: ExprId, cx: &mut FunContext) -> Value {
        match &self.ast[expr] {
            // Literals lower to immediate IR values.
            //
            //   2
            //
            // becomes:
            //
            //   Int(2)
            Expr::Lit(lit) => match lit {
                Lit::Unit => todo!(),
                Lit::Int(n) => Value::Int(*n),
                Lit::Bool(b) => Value::Bool(*b),
            },

            // Resolved variable uses lower to the frame slot chosen for that binding.
            //
            //   x
            //
            // becomes:
            //
            //   Local(l0)
            Expr::Var(_) => {
                let local = self.resolution.uses[expr].unwrap();
                if let Some(&slot) = cx.resolved_locals.get(&local) {
                    Value::Local(slot)
                } else {
                    todo!("captured variables are not lowered yet")
                }
            }

            // A let expression lowers by:
            // 1. lowering the initializer,
            // 2. assigning the binder a frame slot,
            // 3. storing the initializer into that slot,
            // 4. lowering the body with future uses of that binder mapped to the slot.
            //
            // let x = 1 in x + 2
            //
            // l0 = 1
            // t0 = add_int l0, 2
            Expr::Bind {
                is_recursive,
                name,
                init,
                body,
            } => {
                let init = self.lower_expr(*init, cx);
                let bound_local = cx.fresh_local();
                let binder = self.resolution.binders[expr].unwrap();

                cx.resolved_locals.insert(binder, bound_local);
                self.push_instr(
                    cx,
                    Instr::StoreLocal {
                        dst: bound_local,
                        src: init,
                    },
                );

                self.lower_expr(*body, cx)
            }

            Expr::Abs(symbol, id) => todo!(),

            Expr::App(id, id1) => todo!(),

            // Binary expressions lower their operands first, then emit one three-address
            // instruction whose result lives in a fresh temp.
            //
            //   2 + 3
            //
            // becomes:
            //
            //   t0 = add_int 2, 3
            Expr::Bin(lhs, op, rhs) => {
                let lhs = self.lower_expr(*lhs, cx);
                let rhs = self.lower_expr(*rhs, cx);

                let op = match op {
                    Token::Plus => BinOp::AddInt,
                    Token::Minus => BinOp::SubInt,
                    Token::Star => BinOp::MulInt,
                    Token::Slash => BinOp::DivInt,
                    _ => todo!("unsupported binary operator"),
                };

                let dst = cx.fresh_temp();
                let instr = Instr::BinOp { dst, op, lhs, rhs };
                self.push_instr(cx, instr);

                Value::Temp(dst)
            }

            // Conditionals lower to explicit control flow with a join block.
            // Both branches store their result into the same local, then jump to the join.
            //
            // if c then 1 else 2
            //
            // becomes:
            //
            // block0:
            //   branch c, block1, block2
            //
            // block1:
            //   l0 = 1
            //   jump block3
            //
            // block2:
            //   l0 = 2
            //   jump block3
            //
            // block3:
            //   Local(l0)
            Expr::Cond {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond = self.lower_expr(*cond, cx);
                let result_local = cx.fresh_local();
                let then_block = self.fresh_block(cx);
                let else_block = self.fresh_block(cx);
                let join_block = self.fresh_block(cx);

                self.set_terminator(
                    cx,
                    Terminator::Branch {
                        cond,
                        then_block,
                        else_block,
                    },
                );

                cx.current_block = then_block;
                let then_value = self.lower_expr(*then_branch, cx);
                self.push_instr(
                    cx,
                    Instr::StoreLocal {
                        dst: result_local,
                        src: then_value,
                    },
                );
                self.set_terminator(cx, Terminator::Jump(join_block));

                cx.current_block = else_block;
                let else_value = self.lower_expr(*else_branch, cx);
                self.push_instr(
                    cx,
                    Instr::StoreLocal {
                        dst: result_local,
                        src: else_value,
                    },
                );
                self.set_terminator(cx, Terminator::Jump(join_block));

                cx.current_block = join_block;
                Value::Local(result_local)
            }
        }
    }
}
