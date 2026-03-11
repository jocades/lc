fn main() {
    let mut args = std::env::args();
    match args.len() {
        1 => {
            lc::repl();
        }
        2 => {
            let path = args.nth(1).unwrap();
            let source = std::fs::read_to_string(&path).unwrap();
            lc::interpret(&source);
        }
        _ => {
            eprintln!("usage: {} [PATH]", args.nth(0).unwrap());
            std::process::exit(1);
        }
    }
}
