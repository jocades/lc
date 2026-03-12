fn main() {
    let source = r#"
        let b = true in
        let z = if b then
          let x = 2 in x + 1
        else
          let y = 3 in y + 1
        in
        z * 2
    "#;

    lc::interpret(source);
    // lc::interpret("(1 + 2) * 3");

    // lc::interpret("let x = 69 in x + 2");
    // lc::interpret("if true then let x = 1 in x else 2 + 3");
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
