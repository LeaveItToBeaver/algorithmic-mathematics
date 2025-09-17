use std::fs;

use std::path::Path;
use crate::ast::{AlgorithmDef, show_expr};
use crate::error_handling::safe_parse;
use crate::eval::{Env, Value, World, eval_expr};
use crate::lexer::lex;
use crate::normalize::normalize_unicode_to_ascii;
use crate::parser::{Tokens, parse_expr, parse_module};
use crate::resolver::Loader;


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
    let module = parse_module(&mut ts);

    // Set up module resolution
    let file_path = Path::new(&path);
    let parent_dir = file_path.parent().unwrap_or(Path::new("."));
    let search_paths = vec![parent_dir.to_path_buf()];
    let mut loader = Loader::new(search_paths);
    
    // Resolve all dependencies  
    let entry_module = module.clone(); // Keep the original module for symbol table
    let resolved = loader.load_entry(file_path, module)
        .map_err(|e| format!("Module resolution failed: {}", e))?;
    
    let defs = resolved.defs;
    if defs.is_empty() {
        return Err(format!("No algorithms found in {}", path));
    }

    let mut config = FileProcessorConfig::new();
    config.parse_args(&mut args)?;

    if config.print_ast {
        print_ast(&defs);
    }

    if let Some(call_src) = config.call_expr {
        execute_call(&call_src, &defs, &src, &entry_module, &loader.modules)?;
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

fn execute_call(call_src: &str, defs: &[AlgorithmDef], src: &str, entry_module: &crate::ast::Module, all_modules: &std::collections::HashMap<String, crate::ast::Module>) -> Result<(), String> {
    let norm = normalize_unicode_to_ascii(call_src);
    let toks = lex(&norm);
    let mut t2 = Tokens::new_with_src(toks, src);

    let call = safe_parse(|| parse_expr(&mut t2))?;
    let mut world = World::new(defs);
    world.build_symbol_table(entry_module, all_modules);
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
