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
        let captures = captures.into_iter().map(|(&local, _)| local).collect();

        dbg!(&captures);
        let fun = self.funs.len();
        self.funs.push(Fun {
            code: self.code,
            arity,
            captures,
        });
        fun
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

#[derive(Debug, Clone, Copy)]
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
        let Value::Closure(clo) = self else {
            panic!("expected value to be of type `closure` but got `{self}`")
        };
        clo
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
    pub code: Vec<Op>,
    arity: u8,
    captures: Vec<Local>,
}

pub struct Closure {
    pub fun: usize,
    pub captures: Vec<Value>,
}

struct CallFrame {
    closure: usize,
    ip: usize,
    base: usize,
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

    pub fn call(&mut self, clo: usize, argc: u8) {
        let frame = CallFrame {
            closure: clo,
            ip: 0,
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

        loop {
            let frame = self.frames.last_mut().unwrap();
            let op = self.funs[self.closures[frame.closure].fun].code[frame.ip];
            frame.ip += 1;

            println!("  {:?}", self.stack);
            println!("{:02}: {op:?}", frame.ip - 1);

            match op {
                Op::ConstInt(n) => push!(Value::Int(n)),

                Op::ConstBool(b) => self.push(Value::Bool(b)),

                Op::LoadLocal(slot) => {
                    let value = self.stack[frame.base + slot as usize];
                    push!(value);
                }
                Op::LoadCapture(slot) => {
                    let value = self.closures[frame.closure].captures[slot as usize];
                    push!(value);
                }

                Op::Bin(bin_op) => self.exec_bin(bin_op),

                Op::Jmp(target) => {
                    frame.ip = target as usize;
                }
                Op::JmpIfFalse(target) => {
                    if !pop!().as_bool() {
                        frame.ip = target as usize;
                    }
                }

                Op::Closure(fun, capture_count) => {
                    let mut captures = Vec::with_capacity(capture_count as usize);
                    for _ in 0..capture_count {
                        captures.push(pop!());
                    }
                    captures.reverse();
                    let clo = self.closures.len();
                    self.closures.push(Closure { fun, captures });
                    self.push(Value::Closure(clo));
                }

                Op::Call(argc) => {
                    let fun = peek!(argc).as_closure();
                    self.call(fun, argc);
                }
                Op::Ret => {
                    let value = self.pop();
                    println!("RET {value:?}");
                    let frame = self.frames.pop().unwrap();
                    if self.frames.is_empty() {
                        return;
                    }
                    self.stack.truncate(frame.base - 1);
                    self.push(value);
                }
            }
        }
    }
}

#[allow(unused)]
mod opcode {
    pub const CONST_INT: u8 = 1;
    pub const CONST_BOOL: u8 = 2;

    pub const LOAD_LOCAL: u8 = 3;
    pub const LOAD_CAPTURE: u8 = 4;

    pub const ADD_INT: u8 = 5;
    pub const SUB_INT: u8 = 6;
    pub const MUL_INT: u8 = 7;
    pub const DIV_INT: u8 = 8;

    pub const EQ_INT: u8 = 9;
    pub const GT_INT: u8 = 10;
    pub const GE_INT: u8 = 11;
    pub const LT_INT: u8 = 12;
    pub const LE_INT: u8 = 13;

    pub const EQ_BOOL: u8 = 14;

    pub const JMP: u8 = 15;
    pub const JMP_IF_FALSE: u8 = 16;

    pub const CLOSURE: u8 = 17;
    pub const CALL: u8 = 18;
    pub const RET: u8 = 19;
}

#[derive(Default)]
struct Builder {
    buf: Vec<u8>,
}

#[allow(unused)]
impl Builder {
    fn write_i32(&mut self, n: i32) {
        self.buf.extend(n.to_be_bytes());
    }

    fn write_u16(&mut self, n: u16) {
        self.buf.extend(n.to_be_bytes());
    }

    // todo: use constant pool
    pub fn const_int(&mut self, n: i32) {
        self.buf.push(opcode::CONST_INT);
        self.write_i32(n);
    }

    pub fn const_bool(&mut self, b: bool) {
        self.buf.push(opcode::CONST_BOOL);
        self.buf.push(b as u8);
    }

    pub fn load_local(&mut self, slot: u16) {
        self.buf.push(opcode::LOAD_LOCAL);
        self.write_u16(slot);
    }

    pub fn load_capture(&mut self, slot: u16) {
        self.buf.push(opcode::LOAD_CAPTURE);
        self.write_u16(slot);
    }

    pub fn bin(&mut self, op: BinOp) {
        self.buf.push(match op {
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

    pub fn jmp(&mut self, target: u16) {
        self.buf.push(opcode::JMP);
        self.write_u16(target);
    }

    pub fn jmp_if_false(&mut self, target: u16) {
        self.buf.push(opcode::JMP_IF_FALSE);
        self.write_u16(target);
    }

    pub fn closure(&mut self) {}

    pub fn call(&mut self, argc: u8) {
        self.buf.push(opcode::CALL);
        self.buf.push(argc);
    }

    pub fn ret(&mut self) {
        self.buf.push(opcode::RET);
    }
}
