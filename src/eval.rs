/* type Env = HashMap<Symbol, Value>;

#[derive(Debug, Clone)]
enum Value {
    Unit,
    Int(i32),
    Bool(bool),
    Closure(Symbol, Rc<Expr>, RefCell<Env>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Unit => write!(f, "()"),
            Value::Int(n) => write!(f, "{n}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Closure(_, _, _) => write!(f, "<fn>"),
        }
    }
}

struct Context<'a> {
    env: &'a mut Env,
    interner: &'a Interner,
}

fn eval<'a>(cx: &mut Context<'a>, expr: Rc<Expr>) -> Result<Value, String> {
    let value = match expr.as_ref() {
        Expr::Lit(Lit::Unit, _) => Value::Unit,
        Expr::Lit(Lit::Int(int), _) => Value::Int(*int),
        Expr::Lit(Lit::Bool(b), _) => Value::Bool(*b),

        Expr::Var(sym, _) => match cx.env.get(sym) {
            Some(value) => value.clone(),
            None => return Err(format!("unbound variable {}", cx.interner.lookup(*sym))),
        },

        Expr::Abs(param, body, _) => {
            // This obviously performs poorly, copying the entire env hashmap...
            // But this eval is just for testing before compiling into bytecode.
            Value::Closure(*param, body.clone(), RefCell::new(cx.env.clone()))
        }

        Expr::App(lhs, rhs, _) => {
            let Value::Closure(param, body, mut env) = eval(cx, lhs.clone())? else {
                return Err("only closures are callable".into());
            };

            let arg = eval(cx, rhs.clone())?;

            let mut closure_env = env.borrow_mut();
            closure_env.insert(param, arg);

            let mut cx = Context {
                env: &mut closure_env,
                interner: cx.interner,
            };

            return eval(&mut cx, body.clone());
        }

        Expr::Bin(lhs, op, rhs, _) => {
            let (Value::Int(l), Value::Int(r)) = (eval(cx, lhs.clone())?, eval(cx, rhs.clone())?)
            else {
                return Err("operands must be numbers".into());
            };
            match op {
                Token::EqEq => Value::Bool(l == r),
                Token::BangEq => Value::Bool(l != r),
                Token::Plus => Value::Int(l + r),
                Token::Minus => Value::Int(l - r),
                Token::Star => Value::Int(l * r),
                Token::Slash => Value::Int(l / r),
                _ => unreachable!(),
            }
        }

        Expr::Bind {
            is_recursive,
            name,
            init,
            body,
            ..
        } => {
            let init = eval(cx, init.clone())?;
            cx.env.insert(*name, init);
            return eval(cx, body.clone());
        }

        Expr::Cond {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            return match eval(cx, cond.clone())? {
                Value::Bool(true) => eval(cx, then_branch.clone()),
                Value::Bool(false) => eval(cx, else_branch.clone()),
                _ => Err("only booleans are allowed in conditions".into()),
            };
        }
    };

    Ok(value)
} */
