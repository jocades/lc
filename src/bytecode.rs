use std::collections::HashMap;

use crate::{
    arena::Indexer,
    ast::{Ast, AstTable, Expr, ExprId, Lit},
    checker::TypeId,
    lexer::Token,
    resolver::{Local, Resolution},
};

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    AddInt,
    SubInt,
    MulInt,
    DivInt,
    EqInt,
    LtInt,
    LeInt,
    GtInt,
    GeInt,
    EqBool,
}

#[derive(Debug, Clone, Copy)]
pub enum Op {
    ConstInt(i32),
    ConstBool(bool),

    LoadLocal(u16),
    LoadCapture(u16),
    // StoreLocal(u16),
    Bin(BinOp),

    Jmp(u16),
    JmpIfFalse(u16),

    Closure(usize, u8),
    Call(u8),
    Ret,
}

#[derive(Default)]
struct Env<'a> {
    locals: HashMap<Local, u16>,
    captures: HashMap<Local, u16>,
    parent: Option<&'a Env<'a>>,
}

pub struct Emitter<'a> {
    ast: &'a Ast,
    resolution: &'a Resolution,
    types: &'a AstTable<Option<TypeId>>,
    code: Vec<Op>,
    local_count: u16,
    funs: &'a mut Vec<Fun>,
    env: Env<'a>,
}

impl<'a> Emitter<'a> {
    pub fn new(
        ast: &'a Ast,
        resolution: &'a Resolution,
        types: &'a AstTable<Option<TypeId>>,
        funs: &'a mut Vec<Fun>,
    ) -> Self {
        Self {
            ast,
            resolution,
            types,
            code: vec![],
            local_count: 0,
            funs,
            env: Env::default(),
        }
    }

    fn child<'b>(&'b mut self) -> Emitter<'b> {
        Emitter {
            ast: self.ast,
            resolution: self.resolution,
            types: self.types,
            code: vec![],
            local_count: 0,
            funs: self.funs,
            env: Env {
                locals: HashMap::new(),
                captures: HashMap::new(),
                parent: Some(&self.env),
            },
        }
    }

    pub fn emit(mut self, expr: ExprId, arity: u8) -> usize {
        self.emit_expr(expr);
        self.code.push(Op::Ret);

        let mut captures = self.env.captures.iter().collect::<Vec<_>>();
        captures.sort_by_key(|&(_, slot)| slot);
        // let captures = captures.into_iter().map(|(&local, _)| local).collect();

        dbg!(&captures);
        let fun = self.funs.len();
        // self.funs.push(Fun {
        //     code: self.code,
        //     arity,
        //     captures,
        // });
        // fun
        todo!()
    }

    fn fresh_local(&mut self) -> u16 {
        let l = self.local_count;
        self.local_count += 1;
        l
    }

    fn emit_expr(&mut self, expr: ExprId) {
        match self.ast[expr] {
            Expr::Lit(lit) => match lit {
                Lit::Unit => todo!(),
                Lit::Int(n) => self.code.push(Op::ConstInt(n)),
                Lit::Bool(b) => self.code.push(Op::ConstBool(b)),
            },

            Expr::Var(_) => {
                let local = self.resolution.uses[expr].unwrap();

                if let Some(&slot) = self.env.locals.get(&local) {
                    self.code.push(Op::LoadLocal(slot));
                    return;
                }

                if let Some(&slot) = self.env.captures.get(&local) {
                    self.code.push(Op::LoadCapture(slot));
                    return;
                }

                let mut env = self.env.parent.unwrap();
                loop {
                    if env.locals.contains_key(&local) || env.captures.contains_key(&local) {
                        let slot = self.env.captures.len() as u16;
                        self.env.captures.insert(local, slot);
                        self.code.push(Op::LoadCapture(slot));
                        return;
                    }
                    env = env.parent.unwrap();
                }
            }

            Expr::Bind {
                is_recursive,
                name: _,
                init,
                body,
            } => {
                if !is_recursive {
                    self.emit_expr(init);
                    let slot = self.fresh_local();
                    let bound_local = self.resolution.binders[expr].unwrap();
                    self.env.locals.insert(bound_local, slot);
                    self.emit_expr(body);
                } else {
                    todo!("recursive let");
                }
            }

            Expr::Bin(lhs, op, rhs) => {
                self.emit_expr(lhs);
                self.emit_expr(rhs);
                // todo: check types for 'bool' equality
                // let ty = self.types[expr].unwrap();
                let op = match op {
                    Token::Plus => BinOp::AddInt,
                    Token::Minus => BinOp::SubInt,
                    Token::Star => BinOp::MulInt,
                    Token::Slash => BinOp::DivInt,
                    Token::EqEq => BinOp::EqInt,
                    Token::Gt => BinOp::GtInt,
                    Token::GtEq => BinOp::GeInt,
                    Token::Lt => BinOp::LtInt,
                    Token::LtEq => BinOp::LeInt,
                    _ => todo!("unsupported binary operator"),
                };
                self.code.push(Op::Bin(op));
            }

            /*
            if c then 1 else 2

                CONST_BOOL(true)
                JMP_IF_FALSE, <else>
                CONST_INT(1)
                JMP <join>
            <else>:
                CONST_INT(2)
            <join>
                (if expr value on top of the stack)
                ...
            */
            Expr::Cond {
                cond,
                then_branch,
                else_branch,
            } => {
                self.emit_expr(cond);
                let then_jump = self.code.len();
                self.code.push(Op::JmpIfFalse(u16::MAX));
                self.emit_expr(then_branch);
                let else_jump = self.code.len();
                self.code.push(Op::Jmp(u16::MAX));
                self.code[then_jump] = Op::JmpIfFalse(self.code.len() as u16);
                self.emit_expr(else_branch);
                self.code[else_jump] = Op::Jmp(self.code.len() as u16)
            }

            // let add a b = a + b
            // abs(a, abs(b, a + b))
            // fun add(a, b) -> a + b
            Expr::Abs(_, _) => {
                let mut params = vec![];
                let mut body = expr;
                loop {
                    let Expr::Abs(_, inner) = self.ast[body] else {
                        break;
                    };
                    let bound_param = self.resolution.binders[body].unwrap();
                    params.push(bound_param);
                    body = inner;
                }

                println!("params@{} = {params:?}", expr.index());

                let mut child = self.child();

                for binder in &params {
                    let slot = child.fresh_local();
                    child.env.locals.insert(*binder, slot);
                }

                let fun = child.emit(body, params.len() as u8);

                let captures = &self.funs[fun].captures;
                for capture in captures {
                    if let Some(&slot) = self.env.locals.get(&capture) {
                        self.code.push(Op::LoadLocal(slot));
                        continue;
                    }
                    if let Some(&slot) = self.env.captures.get(&capture) {
                        self.code.push(Op::LoadCapture(slot));
                        continue;
                    }
                    unreachable!("capture source not found in enclosing env");
                }

                self.code.push(Op::Closure(fun, captures.len() as u8));
            }

            // let add a b = a + b
            // abs(a, abs(b, a + b))
            // add 2 3
            // app(app)
            Expr::App(_, _) => {
                let mut args = vec![];
                let mut callee = expr;

                while let Expr::App(lhs, arg) = self.ast[callee] {
                    args.push(arg);
                    callee = lhs;
                }

                println!("args@{} = {args:?}", expr.index());

                self.emit_expr(callee);
                for arg in args.iter().rev() {
                    self.emit_expr(*arg);
                }

                self.code.push(Op::Call(args.len() as u8));
            }
            Expr::Error => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Value {
    Int(i32),
    Bool(bool),
    Closure(usize),
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

    fn as_closure(self) -> usize {
        let Value::Closure(fun) = self else {
            panic!("expected value to be of type `closure` but got `{self}`")
        };
        fun
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => n.fmt(f),
            Value::Bool(b) => b.fmt(f),
            Value::Closure(_) => f.write_str("<fn>"),
        }
    }
}

pub struct Fun {
    pub code: Vec<u8>,
    pub arity: u8,
    pub captures: Vec<Local>,
    consts: Vec<Value>,
}

pub struct Closure {
    pub fun: *mut Fun,
    pub captures: Vec<Value>,
}

struct CallFrame {
    closure: *const Closure,
    ip: *mut u8,
    base: usize,
}

impl CallFrame {
    /// Fetch the next byte and advance the instruction pointer.
    fn fetch_byte(&mut self) -> u8 {
        unsafe {
            let byte = *self.ip;
            self.ip = self.ip.add(1);
            byte
        }
    }

    fn fetch_u16(&mut self) -> u16 {
        let hi = self.fetch_byte() as u16;
        let lo = self.fetch_byte() as u16;
        (hi << 8) | lo
    }
}

#[derive(Default)]
pub struct VM {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    pub funs: Vec<Fun>,
    pub closures: Vec<Closure>,
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

    pub fn call(&mut self, clo: usize, argc: u8) {
        let closure = &self.closures[clo];
        let frame = CallFrame {
            closure,
            ip: unsafe { (*closure.fun).code.as_mut_ptr() },
            base: self.stack.len() - argc as usize,
        };
        self.frames.push(frame);
    }

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
        macro_rules! peek {
            ($n:expr) => {
                self.stack[self.stack.len() - 1 - $n as usize]
            };
        }
        macro_rules! bin {
            ($as:path, $op:tt) => {{
                let (lhs, rhs) = self.pop_int2();
                self.push($as(lhs $op rhs));
            }};
        }

        unsafe {
            /* let frame = &raw mut self.frames[self.frames.len() - 1];
            let ip = (*frame).ip;
            let fun = (*(*frame).closure).fun;

            macro_rules! fetch_u8 {
                () => {{
                    let byte = *ip;
                    ip = ip.add(1);
                    byte
                }};
            }

            macro_rules! read_const {
                () => {{}};
            } */

            loop {
                let frame = self.frames.last_mut().unwrap();
                let fun = &*(*frame.closure).fun;
                let opcode = frame.fetch_byte();

                // println!("  {:?}", self.stack);
                // println!(
                //     "{:02}: {}",
                //     *frame.ip.offset(-1) as usize,
                //     opcode::as_str(opcode)
                // );

                use opcode::*;
                match opcode {
                    CONST => {
                        let index = frame.fetch_byte() as usize;
                        let value = fun.consts[index];
                        // let value = (*(*(*frame).closure).fun).consts[index];
                        push!(value);
                    }

                    TRUE => push!(Value::Bool(true)),
                    FALSE => push!(Value::Bool(false)),

                    LOAD_LOCAL => {
                        let slot = frame.fetch_byte() as usize;
                        let value = self.stack[frame.base + slot];
                        push!(value);
                    }
                    LOAD_CAPTURE => {
                        let slot = frame.fetch_byte() as usize;
                        println!("LAOD_CAPTURE {slot}");
                        let value = (&(*frame.closure).captures)[slot];
                        push!(value);
                    }

                    JMP => {
                        // let offset = frame.fetch_u16() as usize;
                        // frame.ip = frame.ip.add(offset);
                    }
                    JMP_IF_FALSE => {
                        let offset = frame.fetch_u16() as usize;
                        if !pop!().as_bool() {
                            frame.ip = frame.ip.add(offset);
                        }
                    }

                    CLOSURE => {
                        let index = frame.fetch_byte() as usize;
                        let fun = fun.consts[index].as_closure();
                        let fun = &mut self.funs[fun];

                        let captures = (0..fun.captures.len())
                            .map(|_| {
                                let is_local = frame.fetch_byte() != 0;
                                let index = frame.fetch_byte() as usize;
                                if is_local {
                                    self.stack[frame.base + index]
                                } else {
                                    (&(*frame.closure).captures)[index]
                                }
                            })
                            .collect();

                        let clo = self.closures.len();
                        self.closures.push(Closure { fun, captures });
                        push!(Value::Closure(clo));
                    }

                    CALL => {
                        let argc = frame.fetch_byte();
                        let clo = peek!(argc).as_closure();
                        self.call(clo, argc);
                    }

                    RET => {
                        let value = pop!();
                        println!("RET {value:?}");
                        let frame = self.frames.pop().unwrap();
                        if self.frames.is_empty() {
                            return;
                        }
                        self.stack.truncate(frame.base - 1);
                        push!(value);
                    }

                    ADD_INT => bin!(Value::Int, +),
                    SUB_INT => bin!(Value::Int, -),
                    MUL_INT => bin!(Value::Int, *),
                    DIV_INT => bin!(Value::Int, /),
                    EQ_INT => bin!(Value::Bool, ==),
                    GT_INT => bin!(Value::Bool, >),
                    GE_INT => bin!(Value::Bool, >=),
                    LT_INT => bin!(Value::Bool, <),
                    LE_INT => bin!(Value::Bool, <=),
                    EQ_BOOL => {
                        let (lhs, rhs) = self.pop_bool2();
                        self.push(Value::Bool(lhs == rhs));
                    }

                    0 | 21..=u8::MAX => unreachable!(),
                    // Op::LoadCapture(slot) => {
                    //     let value = self.closures[frame.closure].captures[slot as usize];
                    //     push!(value);
                    // }

                    // Op::Bin(bin_op) => self.exec_bin(bin_op),
                    // Op::Jmp(target) => {
                    //     frame.ip = target as usize;
                    // }
                    // Op::JmpIfFalse(target) => {
                    //     if !pop!().as_bool() {
                    //         frame.ip = target as usize;
                    //     }
                    // }
                    // Op::Closure(fun, capture_count) => {
                    //     let mut captures = Vec::with_capacity(capture_count as usize);
                    //     for _ in 0..capture_count {
                    //         captures.push(pop!());
                    //     }
                    //     captures.reverse();
                    //     let clo = self.closures.len();
                    //     self.closures.push(Closure { fun, captures });
                    //     self.push(Value::Closure(clo));
                    // }

                    // Op::Call(argc) => {
                    //     let fun = peek!(argc).as_closure();
                    //     self.call(fun, argc);
                    // }
                    // Op::Ret => {
                    //     let value = self.pop();
                    //     println!("RET {value:?}");
                    //     let frame = self.frames.pop().unwrap();
                    //     if self.frames.is_empty() {
                    //         return;
                    //     }
                    //     self.stack.truncate(frame.base - 1);
                    //     self.push(value);
                    // }
                }
            }
        }
    }
}

#[allow(unused)]
mod opcode {
    pub const CONST: u8 = 1;
    pub const TRUE: u8 = 2;
    pub const FALSE: u8 = 3;

    pub const LOAD_LOCAL: u8 = 4;
    pub const LOAD_CAPTURE: u8 = 5;

    pub const ADD_INT: u8 = 6;
    pub const SUB_INT: u8 = 7;
    pub const MUL_INT: u8 = 8;
    pub const DIV_INT: u8 = 9;

    pub const EQ_INT: u8 = 10;
    pub const GT_INT: u8 = 11;
    pub const GE_INT: u8 = 12;
    pub const LT_INT: u8 = 13;
    pub const LE_INT: u8 = 14;

    pub const EQ_BOOL: u8 = 15;

    pub const JMP: u8 = 16;
    pub const JMP_IF_FALSE: u8 = 17;

    pub const CLOSURE: u8 = 18;
    pub const CALL: u8 = 19;
    pub const RET: u8 = 20;

    pub fn as_str(opcode: u8) -> &'static str {
        match opcode {
            CONST => "CONST",
            TRUE => "TRUE",
            FALSE => "FALSE",

            LOAD_LOCAL => "LOAD_LOCAL",
            LOAD_CAPTURE => "LOAD_CAPTURE",

            ADD_INT => "ADD_INT",
            SUB_INT => "SUB_INT",
            MUL_INT => "MUL_INT",
            DIV_INT => "DIV_INT",

            EQ_INT => "EQ_INT",
            GT_INT => "GT_INT",
            GE_INT => "GE_INT",
            LT_INT => "LT_INT",
            LE_INT => "LE_INT",

            EQ_BOOL => "EQ_BOOL",

            JMP => "JMP",
            JMP_IF_FALSE => "JMP_IF_FALSE",

            CLOSURE => "CLOSURE",
            CALL => "CALL",

            RET => "RET",

            0 | 21..=u8::MAX => unreachable!(),
        }
    }
}

#[derive(Default)]
pub struct Builder {
    code: Vec<u8>,
    consts: Vec<Value>,
    consts_map: HashMap<Value, u8>,
}

#[allow(unused)]
impl Builder {
    fn write_u16(&mut self, n: u16) {
        self.code.extend(n.to_be_bytes());
    }

    fn write_bytes(&mut self, byte1: u8, byte2: u8) {
        self.code.push(byte1);
        self.code.push(byte2);
    }

    fn add_const(&mut self, value: Value) -> u8 {
        if let Some(&index) = self.consts_map.get(&value) {
            return index;
        }
        let index = self.consts.len() as u8;
        if index > u8::MAX {
            panic!("too many constants in one chunk");
        }
        self.consts.push(value);
        self.consts_map.insert(value, index);
        index
    }

    pub fn const_int(&mut self, n: i32) {
        self.code.push(opcode::CONST);
        let index = self.add_const(Value::Int(n));
        self.code.push(index);
    }

    pub fn const_bool(&mut self, b: bool) {
        self.code.push(if b { opcode::TRUE } else { opcode::FALSE });
    }

    pub fn load_local(&mut self, slot: u8) {
        self.write_bytes(opcode::LOAD_LOCAL, slot);
    }

    pub fn load_capture(&mut self, slot: u8) {
        self.write_bytes(opcode::LOAD_CAPTURE, slot);
    }

    pub fn bin(&mut self, op: BinOp) {
        self.code.push(match op {
            BinOp::AddInt => opcode::ADD_INT,
            BinOp::SubInt => opcode::SUB_INT,
            BinOp::MulInt => opcode::MUL_INT,
            BinOp::DivInt => opcode::DIV_INT,
            BinOp::EqInt => opcode::EQ_INT,
            BinOp::GtInt => opcode::GT_INT,
            BinOp::GeInt => opcode::GE_INT,
            BinOp::LtInt => opcode::LT_INT,
            BinOp::LeInt => opcode::LE_INT,
            BinOp::EqBool => todo!(),
        });
    }

    pub fn jmp(&mut self) {
        self.code.push(opcode::JMP);
        self.write_bytes(0xff, 0xff);
        // self.write_u16(0xffff);
    }

    pub fn jmp_if_false(&mut self) {
        self.code.push(opcode::JMP_IF_FALSE);
        // self.write_u16(target);
    }

    pub fn call(&mut self, argc: u8) {
        self.code.push(opcode::CALL);
        self.code.push(argc);
    }

    pub fn ret(&mut self) {
        self.code.push(opcode::RET);
    }
}

pub struct Context<'a> {
    pub ast: &'a Ast,
    pub resolution: &'a Resolution,
    pub types: &'a AstTable<Option<TypeId>>,
    pub funs: &'a mut Vec<Fun>,
}

#[derive(Default)]
pub struct E<'a> {
    locals: HashMap<Local, u8>,
    captures: HashMap<Local, u8>,
    parent: Option<&'a E<'a>>,
}

pub struct Em<'a> {
    pub cx: Context<'a>,
    pub bb: Builder,
    pub env: E<'a>,
    pub slot_count: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FunId(usize);

impl<'a> Em<'a> {
    pub fn emit_fun(mut self, expr: ExprId, arity: u8) -> usize {
        self.emit_expr(expr);
        self.bb.ret();

        let mut captures = self.env.captures.iter().collect::<Vec<_>>();
        captures.sort_by_key(|&(_, slot)| slot);
        let captures = captures.into_iter().map(|(&local, _)| local).collect();

        let fun = self.cx.funs.len();
        self.cx.funs.push(Fun {
            code: self.bb.code,
            arity,
            captures,
            consts: self.bb.consts,
        });
        fun
    }

    fn child<'b>(&'b mut self) -> Em<'b> {
        Em {
            cx: Context {
                ast: self.cx.ast,
                resolution: self.cx.resolution,
                types: self.cx.types,
                funs: self.cx.funs,
            },
            bb: Builder::default(),
            env: E {
                parent: Some(&self.env),
                ..Default::default()
            },
            slot_count: 0,
        }
    }

    fn fresh_slot(&mut self) -> u8 {
        let slot = self.slot_count;
        self.slot_count += 1;
        slot
    }

    fn emit_expr(&mut self, expr: ExprId) {
        match self.cx.ast[expr] {
            Expr::Lit(lit) => match lit {
                Lit::Unit => todo!(),
                Lit::Int(n) => self.bb.const_int(n),
                Lit::Bool(b) => self.bb.const_bool(b),
            },

            Expr::Var(_) => {
                let local = self.cx.resolution.uses[expr].unwrap();

                if let Some(&slot) = self.env.locals.get(&local) {
                    self.bb.load_local(slot);
                    return;
                }

                if let Some(&slot) = self.env.captures.get(&local) {
                    self.bb.load_capture(slot);
                    return;
                }

                let mut env = self.env.parent.unwrap();
                loop {
                    if env.locals.contains_key(&local) || env.captures.contains_key(&local) {
                        let slot = self.env.captures.len() as u8;
                        self.env.captures.insert(local, slot);
                        self.bb.load_capture(slot);
                        return;
                    }
                    env = env.parent.unwrap();
                }
            }

            Expr::Bind {
                is_recursive,
                name: _,
                init,
                body,
            } => {
                if !is_recursive {
                    self.emit_expr(init);
                    let slot = self.fresh_slot();
                    let bound_local = self.cx.resolution.binders[expr].unwrap();
                    self.env.locals.insert(bound_local, slot);
                    self.emit_expr(body);
                } else {
                    todo!("recursive let");
                }
            }

            /*
            if c then 1 else 2

                CONST_BOOL(true)
                JMP_IF_FALSE, <else>
                CONST_INT(1)
                JMP <join>
            <else>:
                CONST_INT(2)
            <join>
                (if expr value on top of the stack)
                ...


            00: OP_TRUE
            01: OP_JMP_IF_FALSE
            02: 0xff [jump_to_else_branch]
            03: 0xff
            04: CONST_INT [then_branch]
            05: <const_index>
            06: OP_JUMP [jump_after_else_branch]
            07: 0xff
            08: 0xff
            09: CONST_INT [else_branch]
            10: <const_index>
            11: ...
            */
            Expr::Cond {
                cond,
                then_branch,
                else_branch,
            } => {
                self.emit_expr(cond);
                self.bb.code.push(opcode::JMP_IF_FALSE);
                let then_jump = self.bb.code.len();
                self.bb.write_bytes(0xff, 0xff);
                self.emit_expr(then_branch);

                self.bb.code.push(opcode::JMP);
                let else_jump = self.bb.code.len();
                self.bb.write_bytes(0xff, 0xff);

                let jump = self.bb.code.len() - then_jump - 2; // relative jump
                self.bb.code[then_jump] = ((jump >> 8) & 0xff) as u8;
                self.bb.code[then_jump + 1] = (jump & 0xff) as u8;

                self.emit_expr(else_branch);
                let jump = self.bb.code.len() - else_jump - 2;
                self.bb.code[else_jump] = ((jump >> 8) & 0xff) as u8;
                self.bb.code[else_jump + 1] = (jump & 0xff) as u8;
            }

            Expr::Abs(_, _) => {
                let mut params = vec![];
                let mut body = expr;
                loop {
                    let Expr::Abs(_, inner) = self.cx.ast[body] else {
                        break;
                    };
                    let bound_param = self.cx.resolution.binders[body].unwrap();
                    params.push(bound_param);
                    body = inner;
                }

                let mut child = self.child();

                for binder in &params {
                    let slot = child.fresh_slot();
                    child.env.locals.insert(*binder, slot);
                }

                let fun = child.emit_fun(body, params.len() as u8);

                let captures = &self.cx.funs[fun].captures;
                for capture in captures {
                    if let Some(&slot) = self.env.locals.get(&capture) {
                        self.bb.code.push(1);
                        self.bb.code.push(slot);
                        continue;
                    }
                    if let Some(&slot) = self.env.captures.get(&capture) {
                        self.bb.code.push(0);
                        self.bb.code.push(slot);
                        continue;
                    }
                    unreachable!("captured source not found in enclosing env");
                }

                self.bb.code.push(opcode::CLOSURE);
                let const_index = self.bb.add_const(Value::Closure(fun));
                self.bb.code.push(const_index);
            }
            Expr::App(_, _) => {
                let mut args = vec![];
                let mut callee = expr;

                while let Expr::App(lhs, arg) = self.cx.ast[callee] {
                    args.push(arg);
                    callee = lhs;
                }

                self.emit_expr(callee);

                for arg in args.iter().rev() {
                    self.emit_expr(*arg);
                }

                self.bb.call(args.len() as u8);
            }
            Expr::Bin(lhs, op, rhs) => {
                self.emit_expr(lhs);
                self.emit_expr(rhs);

                // // todo: check types for 'bool' equality
                // // let ty = self.types[expr].unwrap();
                let op = match op {
                    Token::Plus => BinOp::AddInt,
                    Token::Minus => BinOp::SubInt,
                    Token::Star => BinOp::MulInt,
                    Token::Slash => BinOp::DivInt,
                    Token::EqEq => BinOp::EqInt,
                    Token::Gt => BinOp::GtInt,
                    Token::GtEq => BinOp::GeInt,
                    Token::Lt => BinOp::LtInt,
                    Token::LtEq => BinOp::LeInt,
                    _ => todo!("unsupported binary operator"),
                };
                self.bb.bin(op);
            }

            Expr::Error => todo!(),
        }
    }
}
