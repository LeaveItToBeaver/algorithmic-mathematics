use std::env;

mod ast;
mod error_handling;
mod eval;
mod file_processor;
mod lexer;
mod normalize;
mod parser;
mod repl;
mod token;

use file_processor::process_file;
use repl::Repl;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if !args.is_empty() {
        exit_on_error(process_file(args));
        return;
    }

    run_repl();
}

fn exit_on_error(result: Result<(), String>) {
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run_repl() {
    let mut repl = match Repl::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };

    exit_on_error(repl.run());
}
