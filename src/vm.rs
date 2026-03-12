#![allow(unused)]
use std::collections::HashMap;

use crate::{
    ast::{Ast, Expr, ExprId, Lit},
    interner::Interner,
    ir::{self, BinOp, BlockId, Fun},
    lexer::Token,
    parser,
};

#[derive(Debug, Clone, Copy)]
pub enum Op {
    ConstInt(i32),
    ConstBool(bool),

    LoadLocal(u16),
    StoreLocal(u16),

    Bin(BinOp),

    Jump(u16),
    JumpIfFalse(u16),

    Return,
}

/// The result of a compiled function.
pub struct Chunk {
    pub code: Vec<Op>,
    pub local_count: usize,
}

struct Patch {
    at: usize,
    target: BlockId,
    kind: PatchKind,
}

enum PatchKind {
    Jump,
    JumpIfFalse,
}

#[derive(Default)]
struct Emitter {
    code: Vec<Op>,
    block_offsets: HashMap<BlockId, usize>,
    patches: Vec<Patch>,
}

pub fn emit_fun(fun: &Fun) -> Chunk {
    let mut emitter = Emitter::default();

    emitter.emit_blocks(fun);
    emitter.patch_jumps();

    Chunk {
        code: emitter.code,
        local_count: fun.locals as usize,
    }
}

impl Emitter {
    fn emit_blocks(&mut self, fun: &Fun) {
        for block in &fun.blocks {
            self.block_offsets.insert(block.id, self.code.len());

            for instr in &block.instrs {
                self.emit_instr(instr);
            }

            self.emit_terminator(&block.term);
        }
    }

    fn emit_value(&mut self, value: &ir::Value) {
        match *value {
            ir::Value::Int(n) => self.code.push(Op::ConstInt(n)),
            ir::Value::Bool(b) => self.code.push(Op::ConstBool(b)),
            ir::Value::Local(slot) => self.code.push(Op::LoadLocal(slot as u16)),

            ir::Value::Temp(_) => {
                unreachable!("stack bytecode emission should consume temps structurally")
            }

            ir::Value::Env(_) => todo!(),
            ir::Value::Unit => todo!(),
            ir::Value::Fun(_) => todo!(),
        }
    }

    fn emit_instr(&mut self, instr: &ir::Instr) {
        match instr {
            ir::Instr::StoreLocal { dst, src } => {
                // self.emit_value(src);
                self.code.push(Op::StoreLocal(*dst as u16));
            }

            ir::Instr::Move { dst: _, src } => {
                self.emit_value(src);
            }

            ir::Instr::LoadConst { dst, value } => {
                self.emit_value(value);
            }

            ir::Instr::Bin { dst, op, lhs, rhs } => {
                self.emit_value(lhs);
                self.emit_value(rhs);

                self.code.push(Op::Bin(*op));
            }

            ir::Instr::MakeClosure { dst, fun, captures } => todo!(),
            ir::Instr::Call { dst, callee, args } => todo!(),
        }
    }

    fn emit_terminator(&mut self, term: &ir::Terminator) {
        match term {
            ir::Terminator::Return(value) => {
                // Temps are assumed to be on top of the stack.
                if !matches!(value, ir::Value::Temp(_)) {
                    self.emit_value(value);
                }
                self.code.push(Op::Return);
            }

            ir::Terminator::Jump(block_id) => {
                let at = self.code.len();
                self.code.push(Op::Jump(u16::MAX));
                self.patches.push(Patch {
                    at,
                    target: *block_id,
                    kind: PatchKind::Jump,
                });
            }

            ir::Terminator::Branch {
                cond,
                then_block,
                else_block,
            } => {
                self.emit_value(cond);

                let at_join = self.code.len();
                self.code.push(Op::JumpIfFalse(u16::MAX));
                self.patches.push(Patch {
                    at: at_join,
                    target: *else_block,
                    kind: PatchKind::JumpIfFalse,
                });

                let at_then = self.code.len();
                self.code.push(Op::Jump(u16::MAX));
                self.patches.push(Patch {
                    at: at_then,
                    target: *then_block,
                    kind: PatchKind::Jump,
                });
            }

            ir::Terminator::Unset => {
                unreachable!("all blocks must be terminated before bytecode emission");
            }
        }
    }

    fn patch_jumps(&mut self) {
        for patch in &self.patches {
            let target_offset = self.block_offsets[&patch.target];

            match &mut self.code[patch.at] {
                Op::Jump(offset) => *offset = target_offset as u16,
                Op::JumpIfFalse(offset) => *offset = target_offset as u16,
                _ => unreachable!("patch location did not contain jump"),
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Int(i32),
    Bool(bool),
}

/// The types of values are guaranteed to be known at this stage. So panicking
/// when `casting` the value means there is a bug in the pipeline.
impl Value {
    fn as_int(self) -> i32 {
        let Value::Int(n) = self else {
            panic!("expected value to be of type `int` but got `{self}`")
        };
        n
    }

    fn as_bool(self) -> bool {
        let Value::Bool(b) = self else {
            panic!("expected value to be of type `bool` but got `{self}`")
        };
        b
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => n.fmt(f),
            Value::Bool(b) => b.fmt(f),
        }
    }
}

#[derive(Default)]
pub struct VM {
    ip: usize,
    stack: Vec<Value>,
    base: usize,
}

impl VM {
    #[inline]
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    #[inline]
    fn pop(&mut self) -> Value {
        self.stack.pop().expect("vm stack underflow")
    }

    #[inline]
    fn load_local(&self, slot: u16) -> Value {
        self.stack[self.base + slot as usize]
    }

    #[inline]
    fn store_local(&mut self, slot: u16, value: Value) {
        let index = self.base + slot as usize;
        self.stack[index] = value;
    }

    #[inline]
    fn pop_int2(&mut self) -> (i32, i32) {
        let rhs = self.pop().as_int();
        let lhs = self.pop().as_int();
        (lhs, rhs)
    }

    #[inline]
    fn pop_bool2(&mut self) -> (bool, bool) {
        let rhs = self.pop().as_bool();
        let lhs = self.pop().as_bool();
        (lhs, rhs)
    }

    pub fn run(&mut self, chunk: &Chunk) -> Value {
        self.ip = 0;
        self.base = self.stack.len();

        // Preallocate local slots
        for _ in 0..chunk.local_count {
            self.push(Value::Int(0));
        }

        macro_rules! bin {
            ($as:path, $op:tt) => {{
                let (lhs, rhs) = self.pop_int2();
                self.push($as(lhs $op rhs));
            }};
        }

        loop {
            let op = chunk.code[self.ip];
            self.ip += 1;

            println!("{:?}", self.stack);
            println!("{op:?}");

            match op {
                Op::ConstInt(n) => self.push(Value::Int(n)),
                Op::ConstBool(b) => self.push(Value::Bool(b)),

                Op::LoadLocal(slot) => {
                    let value = self.load_local(slot);
                    self.push(value);
                }
                Op::StoreLocal(slot) => {
                    let value = self.pop();
                    self.store_local(slot, value);
                }

                Op::Bin(bin_op) => match bin_op {
                    BinOp::AddInt => bin!(Value::Int, +),
                    BinOp::SubInt => bin!(Value::Int, -),
                    BinOp::MulInt => bin!(Value::Int, *),
                    BinOp::DivInt => bin!(Value::Int, /),
                    BinOp::EqInt => bin!(Value::Bool, ==),
                    BinOp::GtInt => bin!(Value::Bool, >),
                    BinOp::GeInt => bin!(Value::Bool, >=),
                    BinOp::LtInt => bin!(Value::Bool, <),
                    BinOp::LeInt => bin!(Value::Bool, <=),
                    BinOp::EqBool => {
                        let (lhs, rhs) = self.pop_bool2();
                        self.push(Value::Bool(lhs == rhs));
                    }
                },

                Op::Jump(target) => {
                    self.ip = target as usize;
                }
                Op::JumpIfFalse(target) => {
                    if !self.pop().as_bool() {
                        self.ip = target as usize;
                    }
                }

                Op::Return => {
                    let value = self.pop();
                    self.stack.truncate(self.base);
                    return value;
                }
            }
        }
    }
}
