mod ast;
mod interner;
use interner::{Interner, Symbol};

use crate::{arena::Indexer, ir::Lowerer};

mod arena;
mod checker;
mod compiler;
mod diagnostic;
mod ir;
mod lexer;
mod parser;
mod resolver;
mod source;
mod vm;

type PassResult<T> = std::result::Result<T, Vec<diagnostic::Diagnostic>>;

pub fn interpret(source: &str) {
    println!("{source}");

    let mut interner = Interner::with_capacity(1024);

    // let (ast, expr) = parser::parse(source, &mut interner).unwrap();

    // parser::parse(source, &mut interner)
    //     .and_then(|(ast, expr)| {})
    //     .unwrap();

    match parser::parse(source, &mut interner) {
        Ok(Some((ast, expr))) => {
            let resolution = resolver::resolve(&ast, expr, &interner);
            println!("ast:");
            print!("{}", ast.pretty(expr, &interner, &resolution.uses));
        }
        Ok(None) => {}
        Err(diags) => {
            dbg!(&diags);
        }
    };

    // let resolution = resolver::resolve(&ast, expr, &interner);
    // println!("ast:");
    // print!("{}", ast.pretty(expr, &interner, &resolution.uses));

    // println!("checker:");

    // use crate::checker::Checker;
    // let mut checker = Checker::new(&ast, &resolution);
    // checker.infer_top(expr).unwrap();

    // for (id, _) in ast.iter() {
    //     let local = resolution.uses[id];
    //     let ty = checker.table[id].unwrap();
    //     println!("{}: {} | {local:?}", id.index(), checker.type_to_string(ty));
    // }

    // let mut lowerer = Lowerer::new(&ast, &resolution, &checker.table);
    // lowerer.lower(expr);
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

        match parser::parse(&buf, &mut interner) {
            Ok(Some((ast, expr))) => {
                let resolution = resolver::resolve(&ast, expr, &interner);
                println!("ast:");
                print!("{}", ast.pretty(expr, &interner, &resolution.uses));
            }
            Ok(None) => {}
            Err(diags) => {
                dbg!(&diags);
            }
        };

        buf.clear();

        // let Some((ast, expr)) = parser::parse(&buf, &mut interner) else {
        //     buf.clear();
        //     continue;
        // };
        //
        // let resolution = resolver::resolve(&ast, expr, &interner);
        //
        // println!("ast:");
        // println!("{}", ast.pretty(expr, &interner, &resolution.uses));
        //
        // println!("checker:");
        // checker::typecheck(&ast, expr, &resolution);
        //
    }
}
