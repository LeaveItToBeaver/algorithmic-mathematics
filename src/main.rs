use std::env;
use std::fs;
use std::panic::AssertUnwindSafe;

use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

mod ast;
mod eval;
mod lexer;
mod normalize;
mod parser;
mod token;

use ast::{AlgorithmDef, show_expr};
use eval::{Env, Value, World, eval_expr};
use lexer::lex;
use normalize::normalize_unicode_to_ascii;
use parser::{Tokens, parse_alg_def, parse_expr};

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

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();

    // If no arguments provided, start REPL mode
    if args.is_empty() {
        println!("AM Language REPL v0.1.0");
        println!("Type ':help' for commands, 'exit' to quit");

        let mut world_defs: Vec<AlgorithmDef> = Vec::new();

        let mut rl = match DefaultEditor::new() {
            Ok(ed) => ed,
            Err(e) => {
                eprintln!("Failed to start line editor: {e}");
                std::process::exit(1);
            }
        };

        let _ = rl.load_history(".amlang_history");

        loop {
            let line = match rl.readline("repl> ") {
                Ok(s) => s,
                Err(ReadlineError::Interrupted) => {
                    println!("Ctrl-C pressed, exiting...");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("Ctrl-D pressed, exiting...");
                    break;
                }
                Err(e) => {
                    eprintln!("Error reading line: {e}");
                    continue;
                }
            };

            let input = line.trim();
            if input.is_empty() {
                continue; // skip empty lines
            }

            rl.add_history_entry(input).ok();

            if input == "exit" || input == ":q" || input == ":quit" {
                break;
            }
            if input == ":help" {
                println!("Commands:");
                println!("  :help        show this help");
                println!("  :list        list defined algorithms");
                println!("  :reset       clear all definitions");
                println!("  exit, :q     quit");
                continue;
            }
            if input == ":list" {
                if world_defs.is_empty() {
                    println!("<no algorithms defined>");
                } else {
                    for d in &world_defs {
                        println!("{}({})", d.name, d.params.join(", "));
                    }
                }
                continue;
            }
            if input == ":reset" {
                world_defs.clear();
                println!("Definitions cleared.");
                continue;
            }

            // normalize → lex → parse
            let normalized = normalize_unicode_to_ascii(input);
            let tokens = lex(&normalized);

            if tokens.is_empty() {
                continue;
            }

            let mut ts = Tokens::new_with_src(tokens, &normalized);

            if input.starts_with('@') {
                // algorithm definition
                match std::panic::catch_unwind(AssertUnwindSafe(|| parse_alg_def(&mut ts))) {
                    Ok(def) => {
                        // replace if same name already exists
                        if let Some(pos) = world_defs.iter().position(|d| d.name == def.name) {
                            world_defs[pos] = def;
                        } else {
                            world_defs.push(def);
                        }
                        let d = world_defs.last().unwrap();
                        println!("Defined: {}({})", d.name, d.params.join(", "));
                    }
                    Err(e) => {
                        if let Some(msg) = e.downcast_ref::<String>() {
                            eprintln!("{msg}");
                        } else if let Some(msg) = e.downcast_ref::<&str>() {
                            eprintln!("{msg}");
                        } else {
                            eprintln!("Error: invalid algorithm definition");
                        }
                    }
                }
            } else {
                // expression
                match std::panic::catch_unwind(AssertUnwindSafe(|| parse_expr(&mut ts))) {
                    Ok(expr) => {
                        let world = World::new(&world_defs);
                        let mut env = Env::base();
                        match eval_expr(&world, &mut env, &expr) {
                            Ok(Value::Number(n)) => println!("= {}", n),
                            Ok(Value::Bool(b)) => println!("= {}", b),
                            Err(e) => eprintln!("runtime error: {e}"),
                        }
                    }
                    Err(e) => {
                        if let Some(msg) = e.downcast_ref::<String>() {
                            eprintln!("{msg}");
                        } else if let Some(msg) = e.downcast_ref::<&str>() {
                            eprintln!("{msg}");
                        } else {
                            eprintln!("Error: invalid expression");
                        }
                    }
                }
            }
        }

        // save history (ignore errors)
        let _ = rl.save_history(".amlang_history");
        return;
    }

    // File processing mode - arguments provided
    let path = args.remove(0);

    let src_raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("Could not read {}: {}", path, e);
        std::process::exit(1);
    });

    let src = normalize_unicode_to_ascii(&src_raw);
    let tokens = lex(&src);
    let mut ts = Tokens::new_with_src(tokens, &src);
    let defs = parse_all_defs(&mut ts);

    // Default: print first def AST if no flags given
    let mut print_ast = false;
    let mut call_expr: Option<String> = None;

    // Parse remaining flags
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
        let mut t2 = Tokens::new_with_src(toks, &src);

        // We allow either Name(args) or @Name(args) for convenience
        let call = parse_expr(&mut t2);

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
