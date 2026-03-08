mod ast;
mod interner;
use interner::{Interner, Symbol};
mod eval;
mod lexer;
mod parser;
mod source;

pub fn repl() {
    use std::io::{self, BufRead, Write};

    let mut interner = Interner::with_capacity(1024);
    // let mut env = Env::new();

    let mut stdin = io::stdin().lock();
    let mut buf = String::new();
    loop {
        print!("λ> ");
        io::stdout().flush().unwrap();

        if stdin.read_line(&mut buf).unwrap() == 0 {
            break;
        }

        let Some(expr) = parser::parse(&buf, &mut interner) else {
            buf.clear();
            continue;
        };

        println!("{expr:?}");

        /* let mut cx = Context {
            env: &mut env,
            interner: &interner,
        }; */

        /* match eval(&mut cx, Rc::new(expr)) {
            Ok(value) => println!("{value}"),
            Err(reason) => println!("runtime error: {reason}"),
        } */

        // println!("{env:?}");

        buf.clear();
    }
}
