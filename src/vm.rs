#![allow(unused)]
use std::collections::HashMap;

use crate::{
    ast::{Ast, Expr, ExprId, Lit},
    interner::Interner,
    ir::{BlockId, Fun},
    lexer::Token,
    parser,
};

#[derive(Debug, Clone, Copy)]
enum Op {
    ConstInt(i32),
    ConstBool(bool),

    LoadLocal(u16),
    StoreLocal(u16),

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

    Jump(u16),
    JumpIfFalse(u16),

    Return,
}

/// The result of a compiled function.
struct Chunk {
    code: Vec<Op>,
    local_count: usize,
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

fn emit_fun(fun: &Fun) -> Chunk {
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
        }
    }

    fn patch_jumps(&mut self) {}
}

#[derive(Debug, Clone, Copy)]
enum Value {
    Int(i32),
    Bool(bool),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => n.fmt(f),
            Value::Bool(b) => b.fmt(f),
        }
    }
}

pub struct VM {
    interner: Interner,
}
