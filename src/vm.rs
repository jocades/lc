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

fn interpret(code: &[Instruction]) {
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
        interpret(&code);
    }
}
