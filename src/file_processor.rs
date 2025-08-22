use std::fs;

use crate::ast::{AlgorithmDef, show_expr};
use crate::error_handling::safe_parse;
use crate::eval::{Env, Value, World, eval_expr};
use crate::lexer::lex;
use crate::normalize::normalize_unicode_to_ascii;
use crate::parser::{Tokens, parse_expr};

fn parse_all_defs(tokens: &mut Tokens) -> Vec<AlgorithmDef> {
    let mut defs = Vec::new();
    while let Some(t) = tokens.peek() {
        match t {
            crate::token::Token::At => {
                let d = crate::parser::parse_alg_def(tokens);
                defs.push(d);
            }
            _ => break,
        }
    }
    defs
}

struct FileProcessorConfig {
    print_ast: bool,
    call_expr: Option<String>,
}

impl FileProcessorConfig {
    fn new() -> Self {
        Self {
            print_ast: false,
            call_expr: None,
        }
    }

    fn parse_args(&mut self, args: &mut Vec<String>) -> Result<(), String> {
        let mut i = 0;
        while i < args.len() {
            i = self.parse_single_arg(args, i)?;
        }
        Ok(())
    }

    fn parse_single_arg(&mut self, args: &[String], i: usize) -> Result<usize, String> {
        match args[i].as_str() {
            "--ast" => {
                self.print_ast = true;
                Ok(i + 1)
            }
            "--call" => self.parse_call_arg(args, i),
            other => Err(format!("unknown flag: {}", other)),
        }
    }

    fn parse_call_arg(&mut self, args: &[String], i: usize) -> Result<usize, String> {
        if i + 1 >= args.len() {
            return Err("--call requires an expression, e.g. --call \"SafeDiv(1,0)\"".to_string());
        }
        self.call_expr = Some(args[i + 1].clone());
        Ok(i + 2)
    }
}

pub fn process_file(mut args: Vec<String>) -> Result<(), String> {
    let path = args.remove(0);

    let src_raw =
        fs::read_to_string(&path).map_err(|e| format!("Could not read {}: {}", path, e))?;

    let src = normalize_unicode_to_ascii(&src_raw);
    let tokens = lex(&src);
    let mut ts = Tokens::new_with_src(tokens, &src);
    let defs = parse_all_defs(&mut ts);

    if defs.is_empty() {
        return Err(format!("No algorithms found in {}", path));
    }

    let mut config = FileProcessorConfig::new();
    config.parse_args(&mut args)?;

    if config.print_ast {
        print_ast(&defs);
    }

    if let Some(call_src) = config.call_expr {
        execute_call(&call_src, &defs, &src)?;
    } else if !config.print_ast {
        print_summary(&defs, &path);
    }

    Ok(())
}

fn print_ast(defs: &[AlgorithmDef]) {
    for d in defs {
        println!("AlgorithmDef {}({})", d.name, d.params.join(","));
        println!("body:");
        show_expr(&d.body, 1);
    }
}

fn execute_call(call_src: &str, defs: &[AlgorithmDef], src: &str) -> Result<(), String> {
    let norm = normalize_unicode_to_ascii(call_src);
    let toks = lex(&norm);
    let mut t2 = Tokens::new_with_src(toks, src);

    let call = safe_parse(|| parse_expr(&mut t2))?;
    let world = World::new(defs);
    let mut env = Env::base();

    let val = eval_expr(&world, &mut env, &call).map_err(|e| format!("runtime error: {e}"))?;

    match val {
        Value::Number(x) => println!("= {}", x),
        Value::Bool(b) => println!("= {}", b),
    }

    Ok(())
}

fn print_summary(defs: &[AlgorithmDef], path: &str) {
    println!("Loaded {} algorithm(s):", defs.len());
    for d in defs {
        println!("  {}({})", d.name, d.params.join(", "));
    }
    println!(
        "Try:  cargo run -- {} --call \"{}(1,0)\"",
        path, defs[0].name
    );
}
