mod ast;
mod interner;
use interner::{Interner, Symbol};

mod arena;
mod bytecode;
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

            for (i, (id, _)) in ast.iter().enumerate() {
                if let Some(ty) = checker.table[id] {
                    print!("{i:02}: {}", checker.type_to_string(ty));
                    if let Some(b) = resolution.binders[id] {
                        print!(" :binder {}", b.0);
                    }
                    if let Some(u) = resolution.uses[id] {
                        print!(" :use {}", u.0);
                    }
                    println!();
                }
            }

            let mut vm = bytecode::VM::default();
            let cx = bytecode::Context {
                ast: &ast,
                resolution: &resolution,
                types: &checker.table,
                funs: &mut vm.funs,
            };

            let em = bytecode::Em {
                cx,
                bb: bytecode::Builder::default(),
                env: bytecode::E::default(),
                slot_count: 0,
            };
            let fun = em.emit_fun(expr, 0);
            // let emitter = bytecode::Emitter::new(&ast, &resolution, &checker.table, &mut vm.funs);
            // let fun = emitter.emit(expr, 0);

            for (i, fun) in vm.funs.iter().enumerate() {
                println!("=== fn{i} ===");
                println!("arity={} captures={:?}", fun.arity, fun.captures);
                fun.code
                    .iter()
                    .enumerate()
                    .for_each(|(i, op)| println!("{i:02}: {op:?}"));
            }

            let closure = bytecode::Closure {
                fun: &mut vm.funs[fun],
                captures: vec![],
            };

            vm.closures.push(closure);

            vm.call(0, 0);
            vm.run();
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
