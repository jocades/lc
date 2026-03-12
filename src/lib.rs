mod ast;
mod interner;
use interner::{Interner, Symbol};

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

    match parser::parse(source, &mut interner) {
        Ok(Some((ast, expr))) => {
            let resolution = match resolver::resolve(&ast, expr, &interner) {
                Ok(resolution) => resolution,
                Err(diags) => {
                    eprint!("{}", diagnostic::render_all(source, &diags));
                    return;
                }
            };
            println!("ast:");
            print!("{}", ast.pretty(expr, &interner, &resolution.uses));
        }
        Ok(None) => {}
        Err(diags) => {
            eprint!("{}", diagnostic::render_all(source, &diags));
        }
    };
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
                if let Err(diags) = resolver::resolve(&ast, expr, &interner) {
                    eprint!("{}", diagnostic::render_all(&buf, &diags));
                }
            }
            Ok(None) => {}
            Err(diags) => {
                eprint!("{}", diagnostic::render_all(&buf, &diags));
            }
        };

        buf.clear();
    }
}
