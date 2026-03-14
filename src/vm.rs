#![allow(unused)]
use std::collections::HashMap;

use crate::{
    ast::{Ast, Expr, ExprId, Lit},
    interner::Interner,
    ir::{self, BinOp, BlockId},
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

    Call(ir::FunId, u8),

    Return,
}

/// The result of a compiled function.
pub struct Function {
    pub code: Vec<Op>,
    pub arity: u8,
    pub local_count: usize,
}

#[derive(Debug)]
struct Patch {
    at: usize,
    target: BlockId,
    kind: PatchKind,
}

#[derive(Debug)]
enum PatchKind {
    Jump,
    JumpIfFalse,
}

// AST = what the program means syntactically.
// IR = how the program executes.
// bytecode = how the VM consumes that execution plan.
//   basically stackify the recipe that the IR gave us
#[derive(Default)]
struct Emitter {
    code: Vec<Op>,
    block_offsets: HashMap<BlockId, usize>,
    patches: Vec<Patch>,
}

pub fn emit(program: &ir::Program) -> Vec<Function> {
    program.funs.iter().map(emit_fun).collect()
}

pub fn emit_fun(fun: &ir::Fun) -> Function {
    let mut emitter = Emitter::default();

    emitter.emit_blocks(fun);
    emitter.patch_jumps();

    Function {
        code: emitter.code,
        local_count: fun.locals as usize,
        arity: fun.arity as u8,
    }
}

impl Emitter {
    fn emit_blocks(&mut self, fun: &ir::Fun) {
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

            ir::Value::Temp(_) => {} // already on top of the stack

            ir::Value::Env(_) => todo!(),
            ir::Value::Unit => todo!(),
            ir::Value::Fun(fun_id) => {}
        }
    }

    fn emit_instr(&mut self, instr: &ir::Instr) {
        match instr {
            ir::Instr::StoreLocal { dst, src } => {
                self.emit_value(src);
                self.code.push(Op::StoreLocal(*dst as u16));
            }

            ir::Instr::Bin { dst, op, lhs, rhs } => {
                self.emit_value(lhs);
                self.emit_value(rhs);

                self.code.push(Op::Bin(*op));
            }

            ir::Instr::MakeClosure { dst, fun, captures } => todo!(),
            ir::Instr::Call { dst, callee, args } => {
                let ir::Value::Fun(fun_id) = callee else {
                    todo!("only non-capturing lambdas for now, {callee:?}")
                };
                for arg in args {
                    self.emit_value(arg);
                }
                self.code.push(Op::Call(*fun_id, 1));
            }
        }
    }

    fn emit_terminator(&mut self, term: &ir::Terminator) {
        match term {
            ir::Terminator::Return(value) => {
                self.emit_value(value);
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

                let at_cond = self.code.len();
                self.code.push(Op::JumpIfFalse(u16::MAX));
                self.patches.push(Patch {
                    at: at_cond,
                    target: *else_block,
                    kind: PatchKind::JumpIfFalse,
                });

                // let at_then = self.code.len();
                // self.code.push(Op::Jump(u16::MAX));
                // self.patches.push(Patch {
                //     at: at_then,
                //     target: *then_block,
                //     kind: PatchKind::Jump,
                // });

                dbg!(&self.patches);
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

struct CallFrame {
    fun: ir::FunId,
    ip: usize,
    base: usize,
}

#[derive(Default)]
pub struct VM {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    pub functions: Vec<Function>,
    // base: usize,
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

    // #[inline]
    // fn load_local(&self, slot: u16) -> Value {
    //     self.stack[self.base + slot as usize]
    // }
    //
    // #[inline]
    // fn store_local(&mut self, slot: u16, value: Value) {
    //     let index = self.base + slot as usize;
    //     self.stack[index] = value;
    // }

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

    fn exec_bin(&mut self, op: BinOp) {
        macro_rules! bin {
            ($as:path, $op:tt) => {{
                let (lhs, rhs) = self.pop_int2();
                self.push($as(lhs $op rhs));
            }};
        }

        match op {
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
        }
    }

    pub fn call(&mut self, fun_id: ir::FunId, argc: u8) {
        let fun = &self.functions[fun_id as usize];

        let frame = CallFrame {
            fun: fun_id,
            ip: 0,
            base: self.stack.len() - argc as usize,
        };

        for _ in 0..fun.local_count - argc as usize {
            self.stack.push(Value::Int(0));
            // self.push(Value::Int(0));
        }
        self.frames.push(frame);
    }

    // pub fn execute()

    pub fn run(&mut self) {
        macro_rules! push {
            ($value:expr) => {
                self.stack.push($value)
            };
        }
        macro_rules! pop {
            () => {
                self.stack.pop().expect("vm stack underflow")
            };
        }
        // self.ip = 0;
        // self.base = self.stack.len();

        // Preallocate local slots
        // for _ in 0..chunk.local_count {
        //     self.push(Value::Int(0));
        // }

        loop {
            let frame = self.frames.last_mut().unwrap();
            let op = self.functions[frame.fun as usize].code[frame.ip];
            frame.ip += 1;

            println!("  {:?}", self.stack);
            println!("{:02}: {op:?}", frame.ip - 1);

            match op {
                Op::ConstInt(n) => {
                    // self.push(Value::Int(n));
                    push!(Value::Int(n));
                }
                Op::ConstBool(b) => self.push(Value::Bool(b)),

                Op::LoadLocal(slot) => {
                    let value = self.stack[frame.base + slot as usize];
                    push!(value);
                }
                Op::StoreLocal(slot) => {
                    let value = pop!();
                    self.stack[frame.base + slot as usize] = value;
                }

                Op::Bin(bin_op) => self.exec_bin(bin_op),

                Op::Jump(target) => {
                    frame.ip = target as usize;
                }
                Op::JumpIfFalse(target) => {
                    // cannot borrow self as mutable since we are holding a ref to frame
                    // let cond = self.pop().as_bool();
                    if !self.stack.pop().unwrap().as_bool() {
                        frame.ip = target as usize;
                    }
                }

                Op::Call(fun, argc) => {
                    self.call(fun, argc);
                }

                Op::Return => {
                    let value = self.pop();
                    println!("RET {value:?}");
                    let frame = self.frames.pop().unwrap();
                    if self.frames.is_empty() {
                        return;
                    }
                    self.stack.truncate(frame.base);
                    self.stack.push(value);
                }
            }
        }
    }
}
