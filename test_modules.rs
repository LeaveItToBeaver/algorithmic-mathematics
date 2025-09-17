use std::path::PathBuf;
use amlang::resolver::Loader;

fn main() {
    // Test loading Main.am which has imports
    let search_paths = vec![PathBuf::from("examples")];
    let mut loader = Loader::new(search_paths);
    
    // Parse Main.am manually first
    let src = std::fs::read_to_string("examples/Main.am").unwrap();
    let tokens = amlang::lexer::lex(&src);
    let mut ts = amlang::parser::Tokens::new_with_src(tokens, &src);
    let main_module = amlang::parser::parse_module(&mut ts);
    
    println!("Main module: {:#?}", main_module);
    
    // Try to resolve all dependencies
    match loader.load_entry(&PathBuf::from("examples/Main.am"), main_module) {
        Ok(resolved) => {
            println!("\nSuccessfully resolved {} definitions:", resolved.defs.len());
            for def in &resolved.defs {
                println!("  {}", def.name);
            }
        }
        Err(e) => {
            println!("Error resolving: {}", e);
        }
    }
}