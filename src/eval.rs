use core::f64;
use std::collections::HashMap;
use std::fs;

use crate::ast::{AlgorithmDef, BinOp, Expr, MatchArm, Module, Pattern, Type, UnOp, UseItem};
use crate::builtins::call_builtin;

#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Bool(bool),
    String(String),
    Set(Vec<Value>), // Using Vec for ordered iteration; duplicates removed on creation
    List(Vec<Value>),
    /// A closure: captured environment + parameter names + body
    Closure {
        params: Vec<String>,
        body: Expr,
        env: HashMap<String, Value>,
    },
    /// An ADT constructor value: `Some(5)`, `Cons(1, Nil)`
    Constructor {
        name: String,
        values: Vec<Value>,
    },
    /// Unit value (for IO operations that return nothing)
    Unit,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => {
                // Handle NaN specially
                if a.is_nan() && b.is_nan() {
                    true
                } else {
                    a == b
                }
            }
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Set(a), Value::Set(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (
                Value::Constructor {
                    name: n1,
                    values: v1,
                },
                Value::Constructor {
                    name: n2,
                    values: v2,
                },
            ) => n1 == n2 && v1 == v2,
            (Value::Unit, Value::Unit) => true,
            // Closures are never equal (function equality is undecidable)
            (Value::Closure { .. }, Value::Closure { .. }) => false,
            _ => false,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(x) => {
                if x.is_nan() {
                    write!(f, "NaN")
                } else if x.is_infinite() {
                    if *x > 0.0 {
                        write!(f, "∞")
                    } else {
                        write!(f, "-∞")
                    }
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
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "}}")
            }
            Value::List(elems) => {
                write!(f, "[")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "]")
            }
            Value::Closure { params, .. } => {
                write!(f, "<fn({})>", params.join(", "))
            }
            Value::Constructor { name, values } => {
                if values.is_empty() {
                    write!(f, "{}", name)
                } else {
                    write!(f, "{}(", name)?;
                    for (i, v) in values.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", v)?;
                    }
                    write!(f, ")")
                }
            }
            Value::Unit => write!(f, "()"),
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
            Value::Closure { .. } => "function",
            Value::Constructor { .. } => "constructor",
            Value::Unit => "unit",
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
    // known constructor names (from type definitions)
    pub constructors: std::collections::HashSet<String>,
}

impl<'a> World<'a> {
    pub fn new(defs: &'a [AlgorithmDef]) -> Self {
        let mut algs = HashMap::new();
        for d in defs {
            algs.insert(d.name.clone(), d);
        }
        Self {
            algs,
            symbols: HashMap::new(),
            constructors: std::collections::HashSet::new(),
        }
    }

    /// Register constructors from type definitions
    pub fn register_constructors(&mut self, module: &Module) {
        use crate::ast::TopLevelDecl;
        for decl in &module.decls {
            if let TopLevelDecl::TypeDef(typedef) = decl {
                for variant in &typedef.variants {
                    self.constructors.insert(variant.name.clone());
                }
            }
        }
    }

    pub fn build_symbol_table(
        &mut self,
        entry_module: &Module,
        all_modules: &HashMap<String, Module>,
    ) {
        // Register constructors from entry module and imported modules
        self.register_constructors(entry_module);
        for module in all_modules.values() {
            self.register_constructors(module);
        }

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
                    let actual_module = aliases.get(&module_ref).unwrap_or(&module_ref);

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
            if let Some(unqualified) = qualified_name.strip_prefix(&format!("{}.", current_module))
            {
                self.symbols
                    .insert(unqualified.to_string(), qualified_name.clone());
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
    env: &mut Env,
    is_alg: bool,
    name: &str,
    vals: Vec<Value>,
) -> Result<Value, String> {
    // First, check if the name is bound to a closure in the environment
    if let Some(closure) = env.get(name) {
        if let Value::Closure { .. } = closure {
            return apply_closure(world, closure.clone(), vals);
        }
    }

    // Check if it's a known constructor
    if world.constructors.contains(name) {
        return Ok(Value::Constructor {
            name: name.to_string(),
            values: vals,
        });
    }

    // Resolve unqualified names using symbol table
    let resolved_name = world.symbols.get(name).map(|s| s.as_str()).unwrap_or(name);

    // If it's an algorithm (explicit @ or known by name), run that algorithm body
    if is_alg || world.algs.contains_key(resolved_name) {
        let alg = world.algs.get(resolved_name).ok_or_else(|| {
            format!(
                "unknown algorithm: {} (resolved as {})",
                name, resolved_name
            )
        })?;
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
            } else if world.constructors.contains(name) {
                // Nullary constructor (e.g., None)
                Ok(Value::Constructor {
                    name: name.clone(),
                    values: vec![],
                })
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

        // Lambda: capture current environment
        Lambda { params, body } => Ok(Value::Closure {
            params: params.clone(),
            body: (**body).clone(),
            env: env.vars.clone(),
        }),

        // Apply: call a closure with arguments
        Apply { func, args } => {
            let func_val = eval_expr(world, env, func)?;
            let mut arg_vals = Vec::new();
            for a in args {
                arg_vals.push(eval_expr(world, env, a)?);
            }
            apply_closure(world, func_val, arg_vals)
        }

        // Constructor: create a constructor value
        Constructor { name, args } => {
            let mut vals = Vec::new();
            for a in args {
                vals.push(eval_expr(world, env, a)?);
            }
            Ok(Value::Constructor {
                name: name.clone(),
                values: vals,
            })
        }

        // Match: pattern matching
        Match { scrutinee, arms } => {
            let val = eval_expr(world, env, scrutinee)?;
            eval_match(world, env, val, arms)
        }

        // IO: Print
        Print(expr) => {
            let val = eval_expr(world, env, expr)?;
            println!("{}", val);
            Ok(Value::Unit)
        }

        // IO: Read file
        ReadFile(path_expr) => {
            let path = eval_expr(world, env, path_expr)?;
            let path_str = path.as_string()?;
            let content = fs::read_to_string(path_str)
                .map_err(|e| format!("failed to read file '{}': {}", path_str, e))?;
            Ok(Value::String(content))
        }

        // IO: Write file
        WriteFile { path, content } => {
            let path_val = eval_expr(world, env, path)?;
            let path_str = path_val.as_string()?;
            let content_val = eval_expr(world, env, content)?;
            let content_str = match &content_val {
                Value::String(s) => s.clone(),
                other => format!("{}", other),
            };
            fs::write(path_str, &content_str)
                .map_err(|e| format!("failed to write file '{}': {}", path_str, e))?;
            Ok(Value::Unit)
        }
    }
}

/// Apply a closure to arguments
fn apply_closure<'a>(world: &World<'a>, func: Value, args: Vec<Value>) -> Result<Value, String> {
    match func {
        Value::Closure {
            params,
            body,
            env: captured_env,
        } => {
            if params.len() != args.len() {
                return Err(format!(
                    "function expected {} arguments, got {}",
                    params.len(),
                    args.len()
                ));
            }
            // Create new environment with captured env + built-in constants
            let mut new_vars = captured_env;
            // Add built-in constants first (so parameters can shadow them)
            new_vars.insert("inf".to_string(), Value::Number(f64::INFINITY));
            new_vars.insert("NaN".to_string(), Value::Number(f64::NAN));
            new_vars.insert("pi".to_string(), Value::Number(std::f64::consts::PI));
            new_vars.insert("e".to_string(), Value::Number(std::f64::consts::E));
            // Then add parameter bindings (which shadow built-ins if needed)
            for (name, val) in params.iter().zip(args.into_iter()) {
                new_vars.insert(name.clone(), val);
            }

            let mut new_env = Env { vars: new_vars };
            eval_expr(world, &mut new_env, &body)
        }
        other => Err(format!(
            "cannot apply non-function value: {}",
            other.type_name()
        )),
    }
}

/// Evaluate a match expression
fn eval_match<'a>(
    world: &World<'a>,
    env: &mut Env,
    val: Value,
    arms: &[MatchArm],
) -> Result<Value, String> {
    for arm in arms {
        if let Some(bindings) = match_pattern(&arm.pattern, &val) {
            // Create new environment with pattern bindings
            let mut new_env = env.with_binding("_dummy".to_string(), Value::Unit);
            new_env.vars.remove("_dummy");
            for (name, bound_val) in bindings {
                new_env.vars.insert(name, bound_val);
            }
            // Copy existing bindings
            for (k, v) in &env.vars {
                if !new_env.vars.contains_key(k) {
                    new_env.vars.insert(k.clone(), v.clone());
                }
            }
            return eval_expr(world, &mut new_env, &arm.body);
        }
    }
    Err("non-exhaustive match: no pattern matched".to_string())
}

/// Try to match a value against a pattern, returning bindings if successful
fn match_pattern(pattern: &Pattern, val: &Value) -> Option<Vec<(String, Value)>> {
    match pattern {
        Pattern::Wildcard => Some(vec![]),
        Pattern::Var(name) => Some(vec![(name.clone(), val.clone())]),
        Pattern::Number(n) => {
            if let Value::Number(v) = val {
                if (v - n).abs() < 1e-10 {
                    Some(vec![])
                } else {
                    None
                }
            } else {
                None
            }
        }
        Pattern::Bool(b) => {
            if let Value::Bool(v) = val {
                if v == b { Some(vec![]) } else { None }
            } else {
                None
            }
        }
        Pattern::String(s) => {
            if let Value::String(v) = val {
                if v == s { Some(vec![]) } else { None }
            } else {
                None
            }
        }
        Pattern::Constructor { name, args } => {
            if let Value::Constructor {
                name: val_name,
                values,
            } = val
            {
                if name == val_name && args.len() == values.len() {
                    let mut all_bindings = vec![];
                    for (pat, v) in args.iter().zip(values.iter()) {
                        if let Some(bindings) = match_pattern(pat, v) {
                            all_bindings.extend(bindings);
                        } else {
                            return None;
                        }
                    }
                    Some(all_bindings)
                } else {
                    None
                }
            } else {
                None
            }
        }
        Pattern::Tuple(pats) => {
            // Tuples are represented as lists in this simple implementation
            if let Value::List(vals) = val {
                if pats.len() == vals.len() {
                    let mut all_bindings = vec![];
                    for (pat, v) in pats.iter().zip(vals.iter()) {
                        if let Some(bindings) = match_pattern(pat, v) {
                            all_bindings.extend(bindings);
                        } else {
                            return None;
                        }
                    }
                    Some(all_bindings)
                } else {
                    None
                }
            } else {
                None
            }
        }
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
