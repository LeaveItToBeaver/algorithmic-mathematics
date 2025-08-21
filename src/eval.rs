// src/eval.rs
use std::collections::HashMap;

use crate::ast::{AlgorithmDef, BinOp, Expr, UnOp};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
}

impl Value {
    fn as_f64(&self) -> Result<f64, String> {
        match self {
            Value::Number(x) => Ok(*x),
            other => Err(format!("expected number, got {:?}", other)),
        }
    }
    fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            other => Err(format!("expected bool, got {:?}", other)),
        }
    }
}

#[derive(Default)]
pub struct Env {
    // simple variable/constant bindings: a -> 3.0, true -> true, etc.
    vars: HashMap<String, Value>,
}

impl Env {
    pub fn with_params(params: &[String], args: &[Value]) -> Result<Self, String> {
        if params.len() != args.len() {
            return Err(format!(
                "argument count mismatch: expected {}, got {}",
                params.len(),
                args.len()
            ));
        }
        let mut vars = HashMap::new();
        for (p, v) in params.iter().zip(args.iter()) {
            vars.insert(p.clone(), v.clone());
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
        Self { vars }
    }
    fn get(&self, name: &str) -> Option<&Value> {
        self.vars.get(name)
    }
    // fn set(&mut self, name: String, val: Value) {
    //     self.vars.insert(name, val);
    // }
}

pub struct World<'a> {
    // registry of algorithms by name
    pub algs: HashMap<String, &'a AlgorithmDef>,
}

impl<'a> World<'a> {
    pub fn new(defs: &'a [AlgorithmDef]) -> Self {
        let mut algs = HashMap::new();
        for d in defs {
            algs.insert(d.name.clone(), d);
        }
        Self { algs }
    }
}

fn call_name<'a>(
    world: &World<'a>,
    _env: &mut Env,
    is_alg: bool,
    name: &str,
    vals: Vec<Value>,
) -> Result<Value, String> {
    // If it's an algorithm (explicit @ or known by name), run that algorithm body
    if is_alg || world.algs.contains_key(name) {
        let alg = world
            .algs
            .get(name)
            .ok_or_else(|| format!("unknown algorithm: {}", name))?;
        let mut local = Env::with_params(&alg.params, &vals)?;
        return eval_expr(world, &mut local, &alg.body);
    }

    // Otherwise: handle tiny built-in functions here
    match name {
        "sqrt" => {
            if vals.len() != 1 {
                return Err(format!("sqrt expects 1 arg, got {}", vals.len()));
            }
            Ok(Value::Number(vals[0].as_f64()?.sqrt()))
        }
        "abs" => {
            if vals.len() != 1 {
                return Err(format!("abs expects 1 arg, got {}", vals.len()));
            }
            Ok(Value::Number(vals[0].as_f64()?.abs()))
        }
        _ => Err(format!("unknown function: {}", name)),
    }
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
            use BinOp::*;
            let lv = eval_expr(world, env, left)?;
            let rv = eval_expr(world, env, right)?;
            match op {
                Add => Ok(Value::Number(lv.as_f64()? + rv.as_f64()?)),
                Sub => Ok(Value::Number(lv.as_f64()? - rv.as_f64()?)),
                Mul => Ok(Value::Number(lv.as_f64()? * rv.as_f64()?)),
                Div => Ok(Value::Number(lv.as_f64()? / rv.as_f64()?)),
                Eq => Ok(Value::Bool(num_eq(lv.as_f64()?, rv.as_f64()?))),
                Ne => Ok(Value::Bool(!num_eq(lv.as_f64()?, rv.as_f64()?))),
                Lt => Ok(Value::Bool(lv.as_f64()? < rv.as_f64()?)),
                Le => Ok(Value::Bool(lv.as_f64()? <= rv.as_f64()?)),
                Gt => Ok(Value::Bool(lv.as_f64()? > rv.as_f64()?)),
                Ge => Ok(Value::Bool(lv.as_f64()? >= rv.as_f64()?)),
                And => Ok(Value::Bool(lv.as_bool()? && rv.as_bool()?)),
                Or => Ok(Value::Bool(lv.as_bool()? || rv.as_bool()?)),
            }
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
        // @Alg(...) — prepend input as first arg, evaluate the rest, then call
        Call {
            is_alg: true,
            name,
            args,
        } => {
            let mut vals = Vec::with_capacity(1 + args.len());
            vals.push(input);
            for a in args {
                vals.push(eval_expr(world, env, a)?);
            }
            call_name(world, env, true, name, vals)
        }
        // plain function call — same, but is_alg = false
        Call {
            is_alg: false,
            name,
            args,
        } => {
            let mut vals = Vec::with_capacity(1 + args.len());
            vals.push(input);
            for a in args {
                vals.push(eval_expr(world, env, a)?);
            }
            call_name(world, env, false, name, vals)
        }
        // bare identifier in a pipeline: treat as a single-arg call
        Ident(name) => call_name(world, env, false, name, vec![input]),
        other => Err(format!(
            "pipeline step must be a call or name, got {:?}",
            other
        )),
    }
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
