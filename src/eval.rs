use core::f64;
use std::collections::HashMap;

use crate::ast::{AlgorithmDef, BinOp, Expr, Type, UnOp, Module, UseItem};
use crate::builtins::call_builtin;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
    String(String),
    Set(Vec<Value>),   // Using Vec for ordered iteration; duplicates removed on creation
    List(Vec<Value>),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(x) => {
                if x.is_nan() {
                    write!(f, "NaN")
                } else if x.is_infinite() {
                    if *x > 0.0 { write!(f, "∞") } else { write!(f, "-∞") }
                } else if x.fract() == 0.0 && x.abs() < 1e15 {
                    write!(f, "{}", *x as i64)
                } else {
                    write!(f, "{}", x)
                }
            }
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Set(elems) => {
                write!(f, "{{")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", e)?;
                }
                write!(f, "}}")
            }
            Value::List(elems) => {
                write!(f, "[")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", e)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl Value {
    pub fn as_f64(&self) -> Result<f64, String> {
        match self {
            Value::Number(x) => Ok(*x),
            other => Err(format!("expected number, got {}", other.type_name())),
        }
    }
    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            other => Err(format!("expected bool, got {}", other.type_name())),
        }
    }
    pub fn as_string(&self) -> Result<&str, String> {
        match self {
            Value::String(s) => Ok(s),
            other => Err(format!("expected string, got {}", other.type_name())),
        }
    }
    pub fn as_set(&self) -> Result<&Vec<Value>, String> {
        match self {
            Value::Set(v) => Ok(v),
            other => Err(format!("expected set, got {}", other.type_name())),
        }
    }
    pub fn as_list(&self) -> Result<&Vec<Value>, String> {
        match self {
            Value::List(v) => Ok(v),
            other => Err(format!("expected list, got {}", other.type_name())),
        }
    }
    
    /// Get the type name for error messages
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Bool(_) => "bool",
            Value::String(_) => "string",
            Value::Set(_) => "set",
            Value::List(_) => "list",
        }
    }
}

#[derive(Default)]
pub struct Env {
    // simple variable/constant bindings: a -> 3.0, true -> true, etc.
    vars: HashMap<String, Value>,
}

impl Env {
    pub fn with_params(params: &[(String, Option<Type>)], args: &[Value]) -> Result<Self, String> {
        if params.len() != args.len() {
            return Err(format!(
                "argument count mismatch: expected {}, got {}",
                params.len(),
                args.len()
            ));
        }
        let mut vars = HashMap::new();
        for ((name, _ty), v) in params.iter().zip(args.iter()) {
            // TODO: Type checking would go here when we implement it
            vars.insert(name.clone(), v.clone());
        }
        // Built-in constants
        vars.insert("inf".to_string(), Value::Number(f64::INFINITY));
        vars.insert("NaN".to_string(), Value::Number(f64::NAN));
        Ok(Self { vars })
    }
    pub fn base() -> Self {
        let mut vars = HashMap::new();
        vars.insert("inf".to_string(), Value::Number(f64::INFINITY));
        vars.insert("NaN".to_string(), Value::Number(f64::NAN));
        vars.insert("pi".to_string(), Value::Number(std::f64::consts::PI));
        vars.insert("e".to_string(), Value::Number(std::f64::consts::E));
        vars.insert("tau".to_string(), Value::Number(std::f64::consts::TAU));
        Self { vars }
    }
    fn get(&self, name: &str) -> Option<&Value> {
        self.vars.get(name)
    }
    fn set(&mut self, name: String, val: Value) {
        self.vars.insert(name, val);
    }
    
    /// Create a new environment with an additional binding (for let expressions)
    fn with_binding(&self, name: String, val: Value) -> Self {
        let mut new_vars = self.vars.clone();
        new_vars.insert(name, val);
        Self { vars: new_vars }
    }
}

pub struct World<'a> {
    // registry of algorithms by name
    pub algs: HashMap<String, &'a AlgorithmDef>,
    // symbol table: unqualified name -> qualified name (from use statements)
    pub symbols: HashMap<String, String>,
}

impl<'a> World<'a> {
    pub fn new(defs: &'a [AlgorithmDef]) -> Self {
        let mut algs = HashMap::new();
        for d in defs {
            algs.insert(d.name.clone(), d);
        }
        Self { 
            algs,
            symbols: HashMap::new()
        }
    }
    
    pub fn build_symbol_table(&mut self, entry_module: &Module, all_modules: &HashMap<String, Module>) {
        // Build alias map from imports: alias -> module_name
        let mut aliases = HashMap::new();
        for import in &entry_module.imports {
            let module_name = import.path.segments.join(".");
            if let Some(alias) = &import.alias {
                aliases.insert(alias.clone(), module_name);
            } else {
                // If no alias, the module name itself is available
                aliases.insert(module_name.clone(), module_name);
            }
        }
        
        // Build symbol table from use statements
        for use_item in &entry_module.uses {
            match use_item {
                UseItem::Named { path, ident } => {
                    let module_ref = path.segments.join(".");
                    
                    // Resolve alias if present
                    let actual_module = aliases.get(&module_ref)
                        .unwrap_or(&module_ref);
                    
                    let qualified_name = format!("{}.{}", actual_module, ident);
                    self.symbols.insert(ident.clone(), qualified_name);
                }
                UseItem::Star { path: _ } => {
                    // TODO: Handle wildcard imports if needed
                }
            }
        }
        
        // Also add local (current module) functions with unqualified names
        let current_module = &entry_module.name;
        for (qualified_name, _) in &self.algs {
            if let Some(unqualified) = qualified_name.strip_prefix(&format!("{}.", current_module)) {
                self.symbols.insert(unqualified.to_string(), qualified_name.clone());
            }
        }
        
        // Add all exported symbols from imported modules to the symbol table
        for import in &entry_module.imports {
            let module_name = import.path.segments.join(".");
            if let Some(imported_module) = all_modules.get(&module_name) {
                for export_name in &imported_module.exports {
                    let qualified_name = format!("{}.{}", module_name, export_name);
                    if self.algs.contains_key(&qualified_name) {
                        self.symbols.insert(export_name.clone(), qualified_name);
                    }
                }
            }
        }
        
    }
}

fn call_name<'a>(
    world: &World<'a>,
    _env: &mut Env,
    is_alg: bool,
    name: &str,
    vals: Vec<Value>,
) -> Result<Value, String> {
    // Resolve unqualified names using symbol table
    let resolved_name = world.symbols.get(name).map(|s| s.as_str()).unwrap_or(name);
    
    // If it's an algorithm (explicit @ or known by name), run that algorithm body
    if is_alg || world.algs.contains_key(resolved_name) {
        let alg = world
            .algs
            .get(resolved_name)
            .ok_or_else(|| format!("unknown algorithm: {} (resolved as {})", name, resolved_name))?;
        let mut local = Env::with_params(&alg.params, &vals)?;
        return eval_expr(world, &mut local, &alg.body);
    }

    // Otherwise: try built-in mathematical functions
    call_builtin(name, &vals)
}

pub fn eval_expr<'a>(world: &World<'a>, env: &mut Env, e: &Expr) -> Result<Value, String> {
    use Expr::*;
    match e {
        Number(x) => Ok(Value::Number(*x)),
        Bool(b) => Ok(Value::Bool(*b)),
        Ident(name) => {
            if let Some(v) = env.get(name) {
                Ok(v.clone())
            } else {
                Err(format!("unknown identifier: {}", name))
            }
        }
        Unary { op, expr } => {
            let v = eval_expr(world, env, expr)?;
            match op {
                UnOp::Neg => Ok(Value::Number(-v.as_f64()?)),
                UnOp::Not => Ok(Value::Bool(!v.as_bool()?)),
            }
        }
        Bin { op, left, right } => {
            let lv = eval_expr(world, env, left)?;
            let rv = eval_expr(world, env, right)?;
            eval_binary_operation(*op, lv, rv)
        }
        Case { arms, default } => {
            for (cond, rhs) in arms {
                let c = eval_expr(world, env, cond)?;
                if c.as_bool()? {
                    return eval_expr(world, env, rhs);
                }
            }
            eval_expr(world, env, default)
        }
        Call { is_alg, name, args } => {
            // Evaluate arguments to Values
            let mut vals = Vec::with_capacity(args.len());
            for a in args {
                vals.push(eval_expr(world, env, a)?);
            }
            call_name(world, env, *is_alg, name, vals)
        }

        Pipe { head, steps } => {
            // Evaluate head once, then feed through each step
            let mut val = eval_expr(world, env, head)?;
            for step in steps {
                val = apply_step(world, env, step, val)?;
            }
            Ok(val)
        }
        
        // Let binding: evaluate value, extend env, evaluate body
        Let { name, value, body } => {
            let val = eval_expr(world, env, value)?;
            let mut new_env = env.with_binding(name.clone(), val);
            eval_expr(world, &mut new_env, body)
        }
        
        // Set literal: evaluate all elements, deduplicate
        Set(elements) => {
            let mut vals = Vec::new();
            for elem in elements {
                let v = eval_expr(world, env, elem)?;
                // Simple deduplication (keeps first occurrence)
                if !vals.contains(&v) {
                    vals.push(v);
                }
            }
            Ok(Value::Set(vals))
        }
        
        // List literal: evaluate all elements
        List(elements) => {
            let mut vals = Vec::new();
            for elem in elements {
                vals.push(eval_expr(world, env, elem)?);
            }
            Ok(Value::List(vals))
        }
        
        // String literal
        String(s) => Ok(Value::String(s.clone())),
    }
}

fn eval_binary_operation(op: BinOp, lv: Value, rv: Value) -> Result<Value, String> {
    use BinOp::*;
    match op {
        Add => Ok(Value::Number(lv.as_f64()? + rv.as_f64()?)),
        Sub => Ok(Value::Number(lv.as_f64()? - rv.as_f64()?)),
        Mul => Ok(Value::Number(lv.as_f64()? * rv.as_f64()?)),
        Div => Ok(Value::Number(lv.as_f64()? / rv.as_f64()?)),
        Pow => Ok(Value::Number(lv.as_f64()?.powf(rv.as_f64()?))),
        Mod => Ok(Value::Number(lv.as_f64()? % rv.as_f64()?)),
        Eq => Ok(Value::Bool(num_eq(lv.as_f64()?, rv.as_f64()?))),
        Ne => Ok(Value::Bool(!num_eq(lv.as_f64()?, rv.as_f64()?))),
        Lt => Ok(Value::Bool(lv.as_f64()? < rv.as_f64()?)),
        Le => Ok(Value::Bool(lv.as_f64()? <= rv.as_f64()?)),
        Gt => Ok(Value::Bool(lv.as_f64()? > rv.as_f64()?)),
        Ge => Ok(Value::Bool(lv.as_f64()? >= rv.as_f64()?)),
        And => Ok(Value::Bool(lv.as_bool()? && rv.as_bool()?)),
        Or => Ok(Value::Bool(lv.as_bool()? || rv.as_bool()?)),
        Implies => Ok(Value::Bool(!lv.as_bool()? || rv.as_bool()?)),
    }
}

fn apply_step<'a>(
    world: &World<'a>,
    env: &mut Env,
    step: &Expr,
    input: Value,
) -> Result<Value, String> {
    use Expr::*;
    match step {
        Call { is_alg, name, args } => apply_call_step(world, env, *is_alg, name, args, input),
        Ident(name) => call_name(world, env, false, name, vec![input]),
        other => Err(format!(
            "pipeline step must be a call or name, got {:?}",
            other
        )),
    }
}

fn apply_call_step<'a>(
    world: &World<'a>,
    env: &mut Env,
    is_alg: bool,
    name: &str,
    args: &[Expr],
    input: Value,
) -> Result<Value, String> {
    let mut vals = Vec::with_capacity(1 + args.len());
    vals.push(input);
    for a in args {
        vals.push(eval_expr(world, env, a)?);
    }
    call_name(world, env, is_alg, name, vals)
}

fn expect_arity(vals: &[Value], n: usize) -> Result<&[Value], String> {
    if vals.len() != n {
        Err(format!("expected {} argument(s), got {}", n, vals.len()))
    } else {
        Ok(vals)
    }
}

// Equality helper: floating-point equality with NaN handling
fn num_eq(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        true
    } else {
        a == b
    }
}

// Convenience: run an algorithm by name with f64 args
pub fn run_alg(defs: &[AlgorithmDef], name: &str, args: Vec<f64>) -> Result<Value, String> {
    let world = World::new(defs);
    let alg = world
        .algs
        .get(name)
        .ok_or_else(|| format!("no algorithm named {}", name))?;
    let mut env = Env::with_params(
        &alg.params,
        &args.into_iter().map(Value::Number).collect::<Vec<_>>(),
    )?;
    eval_expr(&world, &mut env, &alg.body)
}
