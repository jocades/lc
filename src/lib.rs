mod ast;
mod interner;
use interner::{Interner, Symbol};

use crate::{arena::Indexer, ir::Lowerer};

mod arena;
mod checker;
mod compiler;
mod ir;
mod lexer;
mod parser;
mod resolver;
mod source;
mod vm;

pub fn interpret(source: &str) {
    println!("{source}");

    let mut interner = Interner::with_capacity(1024);

    let (ast, expr) = parser::parse(source, &mut interner).unwrap();

    let resolution = resolver::resolve(&ast, expr, &interner);
    println!("ast:");
    print!("{}", ast.pretty(expr, &interner, &resolution.uses));

    println!("checker:");

    use crate::checker::Checker;
    let mut checker = Checker::new(&ast, &resolution);
    checker.infer_top(expr).unwrap();

    for (id, _) in ast.iter() {
        let local = resolution.uses[id];
        let ty = checker.table[id].unwrap();
        println!("{}: {} | {local:?}", id.index(), checker.type_to_string(ty));
    }

    let mut lowerer = Lowerer::new(&ast, &resolution, &checker.table);
    lowerer.lower(expr);
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

        let resolution = resolver::resolve(&ast, expr, &interner);

        println!("ast:");
        println!("{}", ast.pretty(expr, &interner, &resolution.uses));

        println!("checker:");
        checker::typecheck(&ast, expr, &resolution);

        buf.clear();
    }
}
