fn main() {
    // lc::interpret("(2 + 3) * 4");
    // lc::interpret("2 + 3");
    // lc::interpret("let x = 1 in let y = 2 in x + y");
    // lc::interpret("let x = 1 in x + 2");
    lc::interpret("if true then 1 else 2 + 3");
    // lc::interpret("let id = \\x.x in id 1");

    // let mut args = std::env::args();
    // match args.len() {
    //     1 => {
    //         lc::repl();
    //     }
    //     2 => {
    //         let path = args.nth(1).unwrap();
    //         let source = std::fs::read_to_string(&path).unwrap();
    //         lc::interpret(&source);
    //     }
    //     _ => {
    //         eprintln!("usage: {} [PATH]", args.nth(0).unwrap());
    //         std::process::exit(1);
    //     }
    // }
}
