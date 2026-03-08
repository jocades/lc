mod ast;
mod interner;
use interner::{Interner, Symbol};
mod checker;
mod compiler;
mod eval;
mod lexer;
mod parser;
mod resolver;
mod source;
mod vm;

pub fn interpret(source: &str) {
    let mut interner = Interner::with_capacity(1024);
    let (ast, expr) = parser::parse(source, &mut interner).unwrap();
    // todo: debug expr
    let locals = resolver::resolve(&ast, expr);
    // let printer = AstPrinter {
    //     ast: &ast,
    //     interner: &interner,
    // };
    // printer.print(expr);
    // println!();
    dbg!(locals);
    dbg!(&ast[expr]);
}

use ast::Ast;
struct AstPrinter<'a> {
    ast: &'a Ast,
    interner: &'a Interner,
}

impl<'a> AstPrinter<'a> {
    pub fn string_of_sym(&self, sym: Symbol) -> String {
        format!("{}#{}", self.interner.lookup(sym), sym.0)
    }

    #[allow(unused)]
    pub fn print(&self, expr: ast::ExprId) {
        match &self.ast[expr] {
            ast::Expr::Lit(lit) => print!("{lit:?}"),
            ast::Expr::Var(sym) => print!("Var({})", self.string_of_sym(*sym)),
            ast::Expr::Fun(symbol, expr_id) => todo!(),
            ast::Expr::App(expr_id, expr_id1) => todo!(),
            ast::Expr::Bin(expr_id, token, expr_id1) => todo!(),
            ast::Expr::Bind {
                is_recursive,
                name,
                init,
                body,
            } => todo!(),
            ast::Expr::Cond {
                cond,
                then_branch,
                else_branch,
            } => todo!(),
        }
    }
}

pub fn repl() {
    use std::io::{self, BufRead, Write};
    use vm::VM;

    let mut vm = VM::new();

    let mut stdin = io::stdin().lock();
    let mut buf = String::new();
    loop {
        print!("λ> ");
        io::stdout().flush().unwrap();

        if stdin.read_line(&mut buf).unwrap() == 0 {
            break;
        }

        vm.interpret(&buf);
        buf.clear();

        /* let mut cx = Context {
            env: &mut env,
            interner: &interner,
        }; */

        /* match eval(&mut cx, Rc::new(expr)) {
            Ok(value) => println!("{value}"),
            Err(reason) => println!("runtime error: {reason}"),
        } */

        // println!("{env:?}");
    }
}
