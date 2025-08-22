use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::ast::AlgorithmDef;
use crate::error_handling::safe_parse;
use crate::eval::{Env, Value, World, eval_expr};
use crate::lexer::lex;
use crate::normalize::normalize_unicode_to_ascii;
use crate::parser::{Tokens, parse_alg_def, parse_expr};

pub struct Repl {
        world_defs: Vec<AlgorithmDef>,
        editor: DefaultEditor,
}

impl Repl {
        pub fn new() -> Result<Self, String> {
                let editor = DefaultEditor::new()
                        .map_err(|e| format!("Failed to start line editor: {e}"))?;

                Ok(Self {
                        world_defs: Vec::new(),
                        editor,
                })
        }

        pub fn run(&mut self) -> Result<(), String> {
                println!("AM Language REPL v0.1.0");
                println!("Type ':help' for commands, 'exit' to quit");

                let _ = self.editor.load_history(".amlang_history");

                loop {
                        let line = match self.editor.readline("repl> ") {
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
                                continue;
                        }

                        self.editor.add_history_entry(input).ok();

                        if self.handle_command(input) {
                                continue;
                        }

                        if input == "exit" || input == ":q" || input == ":quit" {
                                break;
                        }

                        self.process_input(input);
                }

                let _ = self.editor.save_history(".amlang_history");
                Ok(())
        }

        fn handle_command(&mut self, input: &str) -> bool {
                match input {
                        ":help" => {
                                println!("Commands:");
                                println!("  :help        show this help");
                                println!("  :list        list defined algorithms");
                                println!("  :reset       clear all definitions");
                                println!("  exit, :q     quit");
                                true
                        }
                        ":list" => {
                                if self.world_defs.is_empty() {
                                        println!("<no algorithms defined>");
                                } else {
                                        for d in &self.world_defs {
                                                println!("{}({})", d.name, d.params.join(", "));
                                        }
                                }
                                true
                        }
                        ":reset" => {
                                self.world_defs.clear();
                                println!("Definitions cleared.");
                                true
                        }
                        _ => false,
                }
        }

        fn process_input(&mut self, input: &str) {
                let normalized = normalize_unicode_to_ascii(input);
                let tokens = lex(&normalized);

                if tokens.is_empty() {
                        return;
                }

                let mut ts = Tokens::new_with_src(tokens, &normalized);

                if input.starts_with('@') {
                        self.handle_algorithm_definition(&mut ts);
                } else {
                        self.handle_expression(&mut ts);
                }
        }

        fn handle_algorithm_definition(&mut self, ts: &mut Tokens) {
                let def = match safe_parse(|| parse_alg_def(ts)) {
                        Ok(def) => def,
                        Err(e) => {
                                eprintln!("{e}");
                                return;
                        }
                };

                self.add_or_replace_algorithm(def);
        }

        fn add_or_replace_algorithm(&mut self, def: AlgorithmDef) {
                if let Some(pos) = self.world_defs.iter().position(|d| d.name == def.name) {
                        self.world_defs[pos] = def;
                } else {
                        self.world_defs.push(def);
                }
                let d = self.world_defs.last().unwrap();
                println!("Defined: {}({})", d.name, d.params.join(", "));
        }

        fn handle_expression(&mut self, ts: &mut Tokens) {
                let expr = match safe_parse(|| parse_expr(ts)) {
                        Ok(expr) => expr,
                        Err(e) => {
                                eprintln!("{e}");
                                return;
                        }
                };

                self.evaluate_and_print_expression(&expr);
        }

        fn evaluate_and_print_expression(&mut self, expr: &crate::ast::Expr) {
                let world = World::new(&self.world_defs);
                let mut env = Env::base();

                match eval_expr(&world, &mut env, expr) {
                        Ok(Value::Number(n)) => println!("= {}", n),
                        Ok(Value::Bool(b)) => println!("= {}", b),
                        Err(e) => eprintln!("runtime error: {e}"),
                }
        }
}
