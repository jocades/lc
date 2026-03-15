fn main() {
    let exmaples = [
        // ("BIND", "let x = 1 in x + 2 * 3"),
        // ("BIN", "2 + 3"),
        // ("COND", "if true then 1 else 2"),
        ("ID", "(\\x.x) 1"),
        // uncurry into fixed fn with N args
        // ("UNCURRY", "let add a b = a + b in add 2 3"),
        // single capture
        // ("SINGLE_CAPTURE", "let x = 1 in let f y = x + y in f 2"),
        // capture parents capture
        // (
        //     "CAPTURE PARENTS CAPTURE",
        //     "let x = 1 in let f y z = x + y + z in f 2 3",
        // ),
        // (
        //     "PAP",
        //     "let add a b = a + b in let add_one b = add 1 in add_one 2",
        // ),
    ];

    for (name, source) in exmaples {
        println!("{name}");
        lc::interpret(source);
    }

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
