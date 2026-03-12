#![allow(unused)]
use crate::{
    ast::{Ast, Expr, ExprId, Lit},
    interner::Interner,
    lexer::Token,
    parser,
};

#[derive(Debug, Clone, Copy)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Copy)]
enum Instruction {
    ConstInt(i32),
    Bin(Op),
    Halt,
    Echo,
}

struct Emitter<'a> {
    ast: &'a Ast,
    code: Vec<Instruction>,
}

impl<'a> Emitter<'a> {
    fn emit(&mut self, id: ExprId) {
        use Instruction::*;

        match &self.ast[id] {
            Expr::Lit(Lit::Int(n)) => self.code.push(ConstInt(*n)),
            Expr::Bin(lhs, op, rhs) => {
                self.emit(*lhs);
                self.emit(*rhs);
                match op {
                    Token::Plus => self.code.push(Bin(Op::Add)),
                    _ => unreachable!(),
                }
            }
            _ => todo!(),
        }
    }

    fn end(mut self) -> Vec<Instruction> {
        self.code.push(Instruction::Halt);
        self.code
    }
}

fn emit(ast: &Ast, expr: ExprId) -> Vec<Instruction> {
    let mut emitter = Emitter {
        ast,
        code: Vec::new(),
    };
    emitter.emit(expr);
    emitter.end()
}

#[derive(Debug, Clone, Copy)]
enum Value {
    Int(i32),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
        }
    }
}

pub struct VM {
    interner: Interner,
}

impl VM {
    pub fn new() -> Self {
        Self {
            interner: Interner::with_capacity(1024),
        }
    }

    // pub fn interpret(&mut self, source: &str) {
    //     let (ast, expr) = parser::parse(source, &mut self.interner).unwrap();
    //     let mut emitter = Emitter {
    //         ast: &ast,
    //         code: Vec::new(),
    //     };
    //     emitter.emit(expr);
    //     let code = emitter.end();
    //     run(&code);
    // }
}

fn run(code: &[Instruction]) {
    let mut ip = 0;
    // Since the types are known we can store a Ve<u8> instead of Vec<Value>
    // Then instructions come along and assumptions are made about what the bytes actually mean.
    let mut stack = [0u8; 30_000];
    let mut sp = 0;

    let store_i32 = |n: i32| {
        stack[sp] = ((n >> 24) & 0xff) as u8;
        stack[sp + 1] = ((n >> 16) & 0xff) as u8;
        stack[sp + 2] = ((n >> 8) & 0xff) as u8;
        stack[sp + 3] = (n & 0xff) as u8;
        sp += 4;
    };

    let load_i32 = || {
        let n = (stack[sp - 4] as i32)
            | (stack[sp - 3] as i32)
            | (stack[sp - 2] as i32)
            | (stack[sp - 1] as i32);
        sp -= 4;
        n
    };

    loop {
        let inst = code[ip];
        ip += 1;

        // println!("{:?}", &stack[..sp]);
        print!("[");
        stack[..sp].iter().for_each(|byte| print!("0x{byte:x}, "));
        println!("]");
        println!("{inst:?}");

        match inst {
            Instruction::ConstInt(n) => {
                // let [h, x, y, l] = n.to_be_bytes();
                // stack[sp] = h;
                // stack[sp + 1] = x;
                // stack[sp + 2] = y;
                // stack[sp + 3] = l;

                stack[sp] = ((n >> 24) & 0xff) as u8;
                stack[sp + 1] = ((n >> 16) & 0xff) as u8;
                stack[sp + 2] = ((n >> 8) & 0xff) as u8;
                stack[sp + 3] = (n & 0xff) as u8;
                sp += 4;
            }
            Instruction::Bin(op) => {
                // 2 + 3
                // [00, 00, 00, 02, 00, 00, 00, 03, 00, 00, 00]
                //  |      a     |  |      b     |   ^

                // let b = ((stack[sp - 4] as i32) << 24)
                //     | ((stack[sp - 3] as i32) << 16)
                //     | ((stack[sp - 2] as i32) << 8)
                //     | (stack[sp - 1] as i32);

                // let x = 1i32 << 24;

                let b = ((stack[sp - 4] as i32) << 24)
                    | ((stack[sp - 3] as i32) << 16)
                    | ((stack[sp - 2] as i32) << 8)
                    | (stack[sp - 1] as i32);
                // println!("0x{x:x}");

                // let b = i32::from_be_bytes([
                //     stack[sp - 4],
                //     stack[sp - 3],
                //     stack[sp - 2],
                //     stack[sp - 1],
                // ]);

                sp -= 4;

                let a = ((stack[sp - 4] as i32) << 24)
                    | ((stack[sp - 3] as i32) << 16)
                    | ((stack[sp - 2] as i32) << 8)
                    | (stack[sp - 1] as i32);
                // let a = i32::from_be_bytes([
                //     stack[sp - 4],
                //     stack[sp - 3],
                //     stack[sp - 2],
                //     stack[sp - 1],
                // ]);

                let n = match op {
                    Op::Add => a + b,
                    Op::Sub => a - b,
                    Op::Mul => a * b,
                    Op::Div => a / b,
                };

                println!("{a} {op:?} {b} = {n}");

                stack[sp - 4] = ((n >> 24) & 0xff) as u8;
                stack[sp - 3] = ((n >> 16) & 0xff) as u8;
                stack[sp - 2] = ((n >> 8) & 0xff) as u8;
                stack[sp - 1] = (n & 0xff) as u8;
                // sp += 4;

                // let b = stack[stack.len() - 4..];
                // let b = i32::from_be_bytes(&stack[stack.len() - 4..stack.len()])
                // let b = i32::from_le_bytes()
                // let Value::Int(b) = stack.pop().unwrap();
                // let Value::Int(a) = stack.pop().unwrap();
                // let result = match op {
                //     Op::Add => a + b,
                //     Op::Sub => a - b,
                //     Op::Mul => a * b,
                //     Op::Div => a / b,
                // };
                // stack.push(Value::Int(result));
            }
            Instruction::Echo => {}
            Instruction::Halt => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add() {
        use Instruction::*;
        let code = vec![ConstInt(0x1234), ConstInt(0x1234), Bin(Op::Add), Echo, Halt];
        run(&code);
    }
}
