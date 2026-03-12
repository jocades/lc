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

            use crate::checker::Checker;
            let mut checker = Checker::new(&ast, &resolution);
            let Ok(_) = checker.infer_top(expr) else {
                return;
            };

            let mut lowerer = ir::Lowerer::new(&ast, &resolution, &checker.table);
            lowerer.lower(expr);

            println!("ir:");
            print!("{}", lowerer.program.pretty());

            dbg!(&lowerer.program);

            let main_fun = &lowerer.program.funs[0];
            let chunk = vm::emit_fun(main_fun);

            println!();
            chunk
                .code
                .iter()
                .enumerate()
                .for_each(|(i, op)| println!("{i:02}: {op:?}"));
            println!();

            use vm::VM;
            let mut vm = VM::default();
            let result = vm.run(&chunk);
            println!("result = {result}");
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
