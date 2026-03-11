mod ast;
mod interner;
use interner::{Interner, Symbol};

mod arena;
mod checker;
mod compiler;
mod lexer;
mod parser;
mod resolver;
mod source;
mod vm;

pub fn interpret(source: &str) {
    println!("{source}");

    let mut interner = Interner::with_capacity(1024);

    let (ast, expr) = parser::parse(source, &mut interner).unwrap();

    let locals = resolver::resolve(&ast, expr, &interner);
    println!("ast:");
    print!("{}", ast.pretty(expr, &interner, &locals));

    println!("checker:");
    checker::typecheck(&ast, expr, &locals);
}

pub fn repl() {
    use std::io::{self, BufRead, Write};

    let mut interner = Interner::with_capacity(1024);

    let mut stdin = io::stdin().lock();
    let mut buf = String::new();
    loop {
        print!("λ> ");
        io::stdout().flush().unwrap();

        if stdin.read_line(&mut buf).unwrap() == 0 {
            break;
        }

        let Some((ast, expr)) = parser::parse(&buf, &mut interner) else {
            buf.clear();
            continue;
        };

        let locals = resolver::resolve(&ast, expr, &interner);

        println!("ast:");
        println!("{}", ast.pretty(expr, &interner, &locals));

        println!("checker:");
        checker::typecheck(&ast, expr, &locals);

        buf.clear();
    }
}
