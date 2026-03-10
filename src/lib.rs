mod ast;
mod interner;
use interner::{Interner, Symbol};
mod checker;
mod compiler;
mod lexer;
mod parser;
mod resolver;
mod source;
mod vm;

pub fn interpret(source: &str) {
    use crate::ast::ExprId;

    println!("{source}");

    let mut interner = Interner::with_capacity(1024);

    let (ast, expr) = parser::parse(source, &mut interner).unwrap();

    // println!("{:?}", ast.nodes);

    let locals = resolver::resolve(&ast, expr, &interner);

    // println!("{locals:?}");
    //
    // for (i, l) in locals.iter().enumerate() {
    //     if let Some(l) = l {
    //         println!("{i}: {l:?} => {:?}", ast[ExprId(i as u32)])
    //     }
    // }

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
        checker::typecheck(&ast, expr, &locals);

        buf.clear();
    }
}
