// src/main.rs
mod ast;
mod eval;
mod lexer;
mod normalize;
mod parser;
mod token;

use std::env;
use std::fs;

use ast::{AlgorithmDef, Expr, show_expr};
use eval::{Env, Value, World, eval_expr, run_alg};
use lexer::lex;
use normalize::normalize_unicode_to_ascii;
use parser::{Tokens, parse_alg_def};
use std::panic;

fn parse_all_defs(tokens: &mut Tokens) -> Vec<AlgorithmDef> {
    let mut defs = Vec::new();
    while let Some(t) = tokens.peek() {
        match t {
            token::Token::At => {
                let d = parse_alg_def(tokens);
                defs.push(d);
            }
            _ => break,
        }
    }
    defs
}

// let res = panic::catch_unwind(|| {
//     let mut ts = parser::Tokens::new(tokens);
//     parse_all_defs(&mut ts);
// });

// let defs = match res {
//     Ok(d) => d,
//     Err(_) => {
//         let err_at =
//     }
// }

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprintln!("Usage: amlang <file.am> [--ast] [--call \"AlgName(1,2)\"]");
        std::process::exit(2);
    }
    let path = args.remove(0);

    let src_raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("Could not read {}: {}", path, e);
        std::process::exit(1);
    });

    let src = normalize_unicode_to_ascii(&src_raw);
    let tokens = lex(&src);
    let mut ts = parser::Tokens::new(tokens);
    let defs = parse_all_defs(&mut ts);

    // Default: print first def AST if no flags given
    let mut print_ast = false;
    let mut call_expr: Option<String> = None;

    // Scrape remaining flags
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--ast" => {
                print_ast = true;
                i += 1;
            }
            "--call" => {
                if i + 1 >= args.len() {
                    eprintln!("--call requires an expression, e.g. --call \"SafeDiv(1,0)\"");
                    std::process::exit(2);
                }
                call_expr = Some(args[i + 1].clone());
                i += 2;
            }
            other => {
                eprintln!("unknown flag: {}", other);
                std::process::exit(2);
            }
        }
    }

    if defs.is_empty() {
        eprintln!("No algorithms found in {}", path);
        std::process::exit(1);
    }

    if print_ast {
        for d in &defs {
            println!("AlgorithmDef {}({})", d.name, d.params.join(","));
            println!("body:");
            show_expr(&d.body, 1);
        }
    }

    if let Some(call_src) = call_expr {
        // Parse the call expression using the same lexer/parser
        let norm = normalize_unicode_to_ascii(&call_src);
        let toks = lex(&norm);
        let mut t2 = parser::Tokens::new(toks);

        // We allow either Name(args) or @Name(args) for convenience
        let call = parser::parse_expr(&mut t2);

        // Evaluate it in a world that knows our algorithm defs
        let world = World::new(&defs);
        let mut env = Env::base();
        let val = eval_expr(&world, &mut env, &call).unwrap_or_else(|e| {
            eprintln!("runtime error: {e}");
            std::process::exit(1)
        });

        match val {
            Value::Number(x) => println!("= {}", x),
            Value::Bool(b) => println!("= {}", b),
        }
    } else if !print_ast {
        // If no flags, just show a summary
        println!("Loaded {} algorithm(s):", defs.len());
        for d in &defs {
            println!("  {}({})", d.name, d.params.join(", "));
        }
        println!(
            "Try:  cargo run -- {} --call \"{}(1,0)\"",
            path, defs[0].name
        );
    }
}
