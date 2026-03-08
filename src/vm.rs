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

    pub fn interpret(&mut self, source: &str) {
        let (ast, expr) = parser::parse(source, &mut self.interner).unwrap();
        let mut emitter = Emitter {
            ast: &ast,
            code: Vec::new(),
        };
        emitter.emit(expr);
        let code = emitter.end();
        run(&code);
    }
}

fn run(code: &[Instruction]) {
    let mut ip = 0;
    let mut stack = Vec::with_capacity(256);

    loop {
        let inst = code[ip];
        ip += 1;

        println!("{stack:?}");
        println!("{inst:?}");

        match inst {
            Instruction::ConstInt(n) => stack.push(Value::Int(n)),
            Instruction::Bin(op) => {
                let Value::Int(b) = stack.pop().unwrap();
                let Value::Int(a) = stack.pop().unwrap();
                let result = match op {
                    Op::Add => a + b,
                    Op::Sub => a - b,
                    Op::Mul => a * b,
                    Op::Div => a / b,
                };
                stack.push(Value::Int(result));
            }
            Instruction::Echo => {
                let value = stack.pop().unwrap();
                println!("{value}");
            }
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
        let code = vec![ConstInt(2), ConstInt(3), Bin(Op::Add), Echo, Halt];
        run(&code);
    }
}
