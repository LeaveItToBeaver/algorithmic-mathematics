use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{AlgorithmDef, Module, TopLevelDecl};

#[derive(Debug)]
pub struct Resolved {
    pub defs: Vec<AlgorithmDef>, // fully-qualified names
}

pub struct Loader {
    pub search_paths: Vec<PathBuf>,
    seen: HashSet<String>,
    pub modules: HashMap<String, Module>,
}

impl Loader {
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths,
            seen: HashSet::new(),
            modules: HashMap::new(),
        }
    }

    pub fn load_entry(&mut self, entry: &Path, parsed: Module) -> Result<Resolved, String> {
        let entry_dir = entry.parent().unwrap_or(Path::new("."));
        self.search_paths.insert(0, entry_dir.to_path_buf());

        let name = parsed.name.clone();
        self.modules.insert(name.clone(), parsed);
        self.visit(&name)?;

        // collect defs with qualified names
        let mut defs = Vec::new();
        for (mname, m) in &self.modules {
            for d in &m.decls {
                if let TopLevelDecl::Definition(def) = d {
                    let mut d2 = def.clone();
                    d2.name = format!("{}.{}", mname, def.name);
                    defs.push(d2);
                }
            }
        }
        
        // Validate exports - ensure all exported symbols actually exist
        self.validate_exports(&defs)?;
        
        Ok(Resolved { defs })
    }

    fn visit(&mut self, name: &str) -> Result<(), String> {
        if !self.seen.insert(name.to_string()) {
            return Ok(()); // already
        }
        
        // If module was already loaded (e.g., entry module), just recurse for its imports
        if let Some(m) = self.modules.get(name).cloned() {
            for imp in m.imports.iter() {
                let dep = imp.path.segments.join(".");
                self.visit(&dep)?;
            }
            return Ok(());
        }
        
        // find file for module: <name>.am in any search path or <name>/mod.am
        let mut tried = Vec::new();
        for base in &self.search_paths {
            let p1 = base.join(format!("{name}.am"));
            let p2 = base.join(name).join("mod.am");
            if p1.exists() {
                let src = fs::read_to_string(&p1).map_err(|e| e.to_string())?;
                let toks = crate::lexer::lex(&src);
                let mut ts = crate::parser::Tokens::new_with_src(toks, &src);
                let m = crate::parser::parse_module(&mut ts);
                self.modules.insert(name.to_string(), m.clone());
                // recurse
                for imp in m.imports.iter() {
                    let dep = imp.path.segments.join(".");
                    self.visit(&dep)?;
                }
                return Ok(());
            }
            if p2.exists() {
                let src = fs::read_to_string(&p2).map_err(|e| e.to_string())?;
                let toks = crate::lexer::lex(&src);
                let mut ts = crate::parser::Tokens::new_with_src(toks, &src);
                let m = crate::parser::parse_module(&mut ts);
                self.modules.insert(name.to_string(), m.clone());
                for imp in m.imports.iter() {
                    let dep = imp.path.segments.join(".");
                    self.visit(&dep)?;
                }
                return Ok(());
            }
            tried.push(p1);
            tried.push(p2);
        }
        Err(format!(
            "Could not locate module '{name}'. Tried: {:?}",
            tried
        ))
    }
    
    fn validate_exports(&self, defs: &[AlgorithmDef]) -> Result<(), String> {
        // Create a set of available definitions by module
        let mut available_defs: HashMap<String, HashSet<String>> = HashMap::new();
        for def in defs {
            if let Some(dot_pos) = def.name.rfind('.') {
                let module_name = &def.name[..dot_pos];
                let def_name = &def.name[dot_pos + 1..];
                available_defs
                    .entry(module_name.to_string())
                    .or_insert_with(HashSet::new)
                    .insert(def_name.to_string());
            }
        }
        
        // Check each module's exports
        for (module_name, module) in &self.modules {
            for export_name in &module.exports {
                if let Some(module_defs) = available_defs.get(module_name) {
                    if !module_defs.contains(export_name) {
                        return Err(format!(
                            "Module '{}' exports '{}' but no such definition exists",
                            module_name, export_name
                        ));
                    }
                } else {
                    return Err(format!(
                        "Module '{}' exports '{}' but the module has no definitions",
                        module_name, export_name
                    ));
                }
            }
        }
        
        Ok(())
    }
}
