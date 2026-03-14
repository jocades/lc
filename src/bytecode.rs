use std::collections::HashMap;

use crate::{
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
    // StoreLocal(u16),
    Bin(BinOp),

    Jmp(u16),
    JmpIfFalse(u16),

    MakeFun(usize),
    Call(u8),
    Ret,
}

pub struct Emitter<'a> {
    ast: &'a Ast,
    resolution: &'a Resolution,
    types: &'a AstTable<Option<TypeId>>,
    code: Vec<Op>,
    resolved_locals: HashMap<Local, u16>,
    local_count: u16,
    funs: &'a mut Vec<Fun>,
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
            resolved_locals: HashMap::new(),
            local_count: 0,
            funs,
        }
    }

    pub fn emit(mut self, expr: ExprId) -> usize {
        self.emit_expr(expr);
        self.code.push(Op::Ret);
        let fun = self.funs.len();
        self.funs.push(Fun {
            code: self.code,
            arity: 1,
        });
        fun
    }

    fn fresh_local(&mut self) -> u16 {
        let l = self.local_count;
        self.local_count += 1;
        l
    }

    fn emit_expr(&mut self, expr: ExprId) {
        match &self.ast[expr] {
            Expr::Lit(lit) => match lit {
                Lit::Unit => todo!(),
                Lit::Int(n) => self.code.push(Op::ConstInt(*n)),
                Lit::Bool(b) => self.code.push(Op::ConstBool(*b)),
            },

            Expr::Var(_) => {
                let local = self.resolution.uses[expr].unwrap();
                if let Some(&slot) = self.resolved_locals.get(&local) {
                    self.code.push(Op::LoadLocal(slot));
                } else {
                    todo!("captured variables");
                }
            }

            Expr::Bind {
                is_recursive,
                name: _,
                init,
                body,
            } => {
                if !is_recursive {
                    self.emit_expr(*init);
                    let slot = self.fresh_local();
                    let bound_local = self.resolution.binders[expr].unwrap();
                    self.resolved_locals.insert(bound_local, slot);
                    self.emit_expr(*body);
                } else {
                    // todo: does not work!
                    let me = self.fresh_local();
                    let bound_local = self.resolution.binders[expr].unwrap();
                    self.resolved_locals.insert(bound_local, me);
                    self.emit_expr(*init);
                    self.emit_expr(*body);
                }
            }

            Expr::Bin(lhs, op, rhs) => {
                self.emit_expr(*lhs);
                self.emit_expr(*rhs);
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
                self.emit_expr(*cond);
                let then_jump = self.code.len();
                self.code.push(Op::JmpIfFalse(u16::MAX));
                self.emit_expr(*then_branch);
                let else_jump = self.code.len();
                self.code.push(Op::Jmp(u16::MAX));
                self.code[then_jump] = Op::JmpIfFalse(self.code.len() as u16);
                self.emit_expr(*else_branch);
                self.code[else_jump] = Op::Jmp(self.code.len() as u16)
            }

            Expr::Abs(_, body) => {
                // no captures for now
                let mut emitter = Emitter::new(self.ast, self.resolution, self.types, self.funs);

                let slot = emitter.fresh_local();
                let bound_param = emitter.resolution.binders[expr].unwrap();
                emitter.resolved_locals.insert(bound_param, slot);

                let fun = emitter.emit(*body);
                self.code.push(Op::MakeFun(fun));
            }

            Expr::App(lhs, arg) => {
                self.emit_expr(*lhs);
                self.emit_expr(*arg);
                self.code.push(Op::Call(1));
            }
            Expr::Error => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Value {
    Int(i32),
    Bool(bool),
    Fun(usize),
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

    fn as_fun(self) -> usize {
        let Value::Fun(fun) = self else {
            panic!("expected value to be of type `bool` but got `{self}`")
        };
        fun
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => n.fmt(f),
            Value::Bool(b) => b.fmt(f),
            Value::Fun(_) => f.write_str("<fn>"),
        }
    }
}

pub struct Fun {
    pub code: Vec<Op>,
    arity: u8,
}

struct CallFrame {
    fun: usize,
    ip: usize,
    base: usize,
}

#[derive(Default)]
pub struct VM {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    pub funs: Vec<Fun>,
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

    pub fn call(&mut self, fid: usize, argc: u8) {
        let frame = CallFrame {
            fun: fid,
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

        loop {
            let frame = self.frames.last_mut().unwrap();
            let op = self.funs[frame.fun as usize].code[frame.ip];
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
                /* Op::StoreLocal(slot) => {
                    let value = pop!();
                    self.stack[frame.base + slot as usize] = value;
                } */
                Op::Bin(bin_op) => self.exec_bin(bin_op),

                Op::Jmp(target) => {
                    frame.ip = target as usize;
                }
                Op::JmpIfFalse(target) => {
                    if !self.stack.pop().unwrap().as_bool() {
                        frame.ip = target as usize;
                    }
                }

                Op::MakeFun(fun) => self.stack.push(Value::Fun(fun)),

                Op::Call(argc) => {
                    let fun = self.stack[self.stack.len() - 1 - argc as usize].as_fun();
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
                    self.stack.push(value);
                }
            }
        }
    }
}
