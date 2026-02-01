use std::collections::{HashMap, HashSet};
use std::process::Command;

use crate::ast::{AlgorithmDef, BinOp, Expr, Specification, UnOp};
use crate::eval::{Value, World};

/// Proof mode: run an external checker (SymPy via python3) on a concrete call.
///
/// This is intentionally pragmatic:
/// - Uses exact integers for literals/arguments.
/// - Executes algorithms symbolically in SymPy.
/// - If `requires`/`ensures` exist, checks them as boolean goals.
///
/// If SymPy isn't installed, we return a helpful error.
pub fn prove_call_with_sympy<'a>(
    world: &World<'a>,
    defs: &'a [AlgorithmDef],
    call: &Expr,
) -> Result<(), String> {
    let (call_name, args) = match call {
        Expr::Call { name, args, .. } => (name.as_str(), args.as_slice()),
        other => {
            return Err(format!(
                "proof mode currently supports proving a direct call expression like Name(1,2); got: {:?}",
                other
            ));
        }
    };

    // Resolve the call name using the symbol table (unqualified -> qualified).
    let resolved_name = world
        .symbols
        .get(call_name)
        .map(|s| s.as_str())
        .unwrap_or(call_name);

    // Build a map of qualified algorithm defs.
    let mut algs: HashMap<&str, &AlgorithmDef> = HashMap::new();
    for d in defs {
        algs.insert(d.name.as_str(), d);
    }

    let target = algs.get(resolved_name).ok_or_else(|| {
        format!("proof mode: unknown algorithm '{call_name}' (resolved as '{resolved_name}')")
    })?;

    // Evaluate call arguments in *run* semantics only to get concrete values to feed SymPy.
    // We require them to be integers so proof-mode stays exact.
    let mut run_env = crate::eval::Env::base();
    let mut arg_vals: Vec<Value> = Vec::with_capacity(args.len());
    for a in args {
        let v = crate::eval::eval_expr(world, &mut run_env, a)
            .map_err(|e| format!("proof mode: couldn't evaluate argument: {e}"))?;
        match v {
            Value::Number(x) if is_int_like(x) => arg_vals.push(Value::Number(x.round())),
            Value::Number(x) => {
                return Err(format!(
                    "proof mode currently requires integer arguments (got {x}). Use --mode run for floats."
                ));
            }
            other => {
                return Err(format!(
                    "proof mode currently requires numeric arguments (got {}).",
                    other.type_name()
                ));
            }
        }
    }

    let script = build_sympy_script(world, defs, target, &arg_vals)?;

    let python = std::env::var("AMLANG_PYTHON").unwrap_or_else(|_| "python3".to_string());

    let out = Command::new(&python)
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            {
                let mut stdin = child.stdin.take().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::Other, "failed to open stdin")
                })?;
                stdin.write_all(script.as_bytes())?;
            }
            child.wait_with_output()
        })
        .map_err(|e| {
            format!("proof mode requires Python (and SymPy). Failed to run '{python}': {e}")
        })?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        // Give a nice hint for the most common failure.
        if stderr.contains("No module named") && stderr.contains("sympy") {
            return Err(
                "proof mode requires SymPy. Install with: python3 -m pip install sympy".to_string(),
            );
        }
        return Err(format!(
            "proof mode checker failed (exit {}).\n{}",
            out.status.code().unwrap_or(1),
            stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    // Expected protocol: one RESULT line + one PROOF line.
    let result_line = stdout.lines().find(|l| l.starts_with("RESULT="));
    let proof_line = stdout.lines().find(|l| l.starts_with("PROOF="));

    if let Some(line) = result_line {
        println!("= {}", &line["RESULT=".len()..]);
    }
    if let Some(line) = proof_line {
        let status = &line["PROOF=".len()..];
        if status.trim() == "OK" {
            println!("Proof: OK (checked by SymPy)");
        } else {
            println!("Proof: {}", status.trim());
        }
    } else {
        // If script didn't print the sentinel, show stdout for debugging.
        println!("Proof: (no status returned)\n{}", stdout);
    }

    Ok(())
}

fn is_int_like(x: f64) -> bool {
    (x.fract().abs() < 1e-12) && x.is_finite() && x.abs() < 9e15
}

fn build_sympy_script<'a>(
    world: &World<'a>,
    defs: &'a [AlgorithmDef],
    entry_alg: &'a AlgorithmDef,
    entry_args: &[Value],
) -> Result<String, String> {
    let mut alg_map: HashMap<&str, &AlgorithmDef> = HashMap::new();
    for d in defs {
        alg_map.insert(d.name.as_str(), d);
    }

    // Reachability: include all algorithms (small project) to keep it simple.
    // Later we can shrink this by walking the call graph.
    let mut py = String::new();
    py.push_str("import sympy as sp\n");
    py.push_str("from sympy import Eq\n");
    py.push_str("\n");

    // Emit python functions only for algorithms reachable from the entry call.
    // This avoids failing on unrelated example programs that might contain free identifiers.
    let reachable = collect_reachable_algorithms(world, &alg_map, entry_alg.name.as_str())?;

    // Use a stable mapping from qualified name to python function name.
    let mut emitted: HashSet<&str> = HashSet::new();
    for name in &reachable {
        let d = alg_map
            .get(name.as_str())
            .ok_or_else(|| format!("proof mode: internal error: missing def for '{name}'"))?;
        emit_alg_function(&mut py, world, &alg_map, d, &mut emitted)?;
    }

    // Bind entry parameters at module scope (so specs can refer to param names).
    // Then call entry algorithm with those bindings.
    let entry_py_name = py_fn_name(&entry_alg.name);
    let mut call_args: Vec<String> = Vec::with_capacity(entry_args.len());
    for ((param_name, _), v) in entry_alg.params.iter().zip(entry_args.iter()) {
        let py_param = sanitize_ident(param_name);
        let rhs = match v {
            Value::Number(x) => format!("sp.Integer({})", *x as i64),
            other => {
                return Err(format!(
                    "proof mode currently only supports integer numeric args; got {}",
                    other.type_name()
                ));
            }
        };
        py.push_str(&format!("{py_param} = {rhs}\n"));
        call_args.push(py_param);
    }

    py.push_str("\n");
    py.push_str(&format!(
        "result = {entry_py_name}({})\n",
        call_args.join(", ")
    ));

    // If the function has a spec, check it.
    if let Some(spec) = &entry_alg.spec {
        emit_spec_checks(&mut py, world, &alg_map, entry_alg, spec)?;
    }

    py.push_str("print('RESULT=' + sp.sstr(result))\n");
    py.push_str("print('PROOF=OK')\n");

    Ok(py)
}

fn collect_reachable_algorithms<'a>(
    world: &World<'a>,
    algs: &HashMap<&'a str, &'a AlgorithmDef>,
    entry_name: &'a str,
) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();

    fn visit<'a>(
        world: &World<'a>,
        algs: &HashMap<&'a str, &'a AlgorithmDef>,
        name: &'a str,
        seen: &mut HashSet<&'a str>,
        out: &mut Vec<String>,
    ) -> Result<(), String> {
        if !seen.insert(name) {
            return Ok(());
        }
        out.push(name.to_string());
        let def = algs
            .get(name)
            .ok_or_else(|| format!("proof mode: missing algorithm def for '{name}'"))?;

        let mut callees: Vec<String> = Vec::new();
        collect_callees(world, algs, &def.body, &mut callees);
        for callee in callees {
            // We can only recurse if callee is in the alg map. It's ok if it isn't (builtin).
            if let Some(def) = algs.get(callee.as_str()) {
                let key: &'a str = def.name.as_str();
                visit(world, algs, key, seen, out)?;
            }
        }
        Ok(())
    }

    fn collect_callees<'a>(
        world: &World<'a>,
        algs: &HashMap<&'a str, &'a AlgorithmDef>,
        e: &Expr,
        out: &mut Vec<String>,
    ) {
        match e {
            Expr::Call { is_alg, name, args } => {
                // Resolve possible algorithm reference.
                let resolved = world.symbols.get(name).map(|s| s.as_str()).unwrap_or(name);
                if *is_alg || algs.contains_key(resolved) {
                    out.push(resolved.to_string());
                }
                for a in args {
                    collect_callees(world, algs, a, out);
                }
            }
            Expr::Unary { expr, .. } => collect_callees(world, algs, expr, out),
            Expr::Bin { left, right, .. } => {
                collect_callees(world, algs, left, out);
                collect_callees(world, algs, right, out);
            }
            Expr::Case { arms, default } => {
                for (c, v) in arms {
                    collect_callees(world, algs, c, out);
                    collect_callees(world, algs, v, out);
                }
                collect_callees(world, algs, default, out);
            }
            Expr::Pipe { head, steps } => {
                collect_callees(world, algs, head, out);
                for s in steps {
                    collect_callees(world, algs, s, out);
                }
            }
            Expr::Let { value, body, .. } => {
                collect_callees(world, algs, value, out);
                collect_callees(world, algs, body, out);
            }
            Expr::Set(xs) | Expr::List(xs) => {
                for x in xs {
                    collect_callees(world, algs, x, out);
                }
            }
            Expr::Number(_) | Expr::Bool(_) | Expr::String(_) | Expr::Ident(_) => {}
        }
    }

    // entry_name is already qualified.
    visit(world, algs, entry_name, &mut seen, &mut out)?;
    Ok(out)
}

fn emit_alg_function<'a>(
    py: &mut String,
    world: &World<'a>,
    algs: &HashMap<&'a str, &'a AlgorithmDef>,
    def: &'a AlgorithmDef,
    emitted: &mut HashSet<&'a str>,
) -> Result<(), String> {
    if !emitted.insert(def.name.as_str()) {
        return Ok(());
    }

    let fn_name = py_fn_name(&def.name);
    let params = def
        .params
        .iter()
        .map(|(n, _)| sanitize_ident(n))
        .collect::<Vec<_>>()
        .join(", ");

    py.push_str(&format!("def {fn_name}({params}):\n"));

    // Environment maps AM identifiers to python expressions.
    let mut env: HashMap<String, String> = HashMap::new();
    for (n, _) in &def.params {
        env.insert(n.clone(), sanitize_ident(n));
    }
    // Builtin constants
    env.insert("pi".to_string(), "sp.pi".to_string());
    env.insert("e".to_string(), "sp.E".to_string());
    env.insert("tau".to_string(), "sp.tau".to_string());
    env.insert("i".to_string(), "sp.I".to_string());
    env.insert("inf".to_string(), "sp.oo".to_string());
    env.insert("NaN".to_string(), "sp.nan".to_string());

    let mut body_lines: Vec<String> = Vec::new();
    let result_expr = translate_expr(world, algs, &def.body, &mut env, &mut body_lines)?;

    for line in body_lines {
        py.push_str("    ");
        py.push_str(&line);
        py.push('\n');
    }
    py.push_str(&format!("    return {result_expr}\n\n"));
    Ok(())
}

fn emit_spec_checks<'a>(
    py: &mut String,
    world: &World<'a>,
    algs: &HashMap<&'a str, &'a AlgorithmDef>,
    def: &'a AlgorithmDef,
    spec: &'a Specification,
) -> Result<(), String> {
    // Reconstruct param environment (params are in scope in python at call site only if we
    // also re-bind them. So we re-bind by unpacking from function signature is not available.
    // Instead: specs currently checked only for the entry call, and we rely on the entry
    // call being evaluated in the module scope with concrete args.

    // We can still translate spec expressions because they reference params by name and `result`.
    let mut env: HashMap<String, String> = HashMap::new();
    for (n, _) in &def.params {
        env.insert(n.clone(), sanitize_ident(n));
    }
    env.insert("result".to_string(), "result".to_string());
    env.insert("pi".to_string(), "sp.pi".to_string());
    env.insert("e".to_string(), "sp.E".to_string());
    env.insert("tau".to_string(), "sp.tau".to_string());
    env.insert("i".to_string(), "sp.I".to_string());
    env.insert("inf".to_string(), "sp.oo".to_string());
    env.insert("NaN".to_string(), "sp.nan".to_string());

    // requires
    for req in &spec.requires {
        let mut stmts = Vec::new();
        let req_py = translate_expr(world, algs, req, &mut env, &mut stmts)?;
        for s in stmts {
            py.push_str(&s);
            py.push('\n');
        }
        py.push_str(&format!(
            "if not bool(sp.simplify({req_py})):\n    raise SystemExit('PROOF=FAIL (requires)')\n"
        ));
    }

    // ensures
    for ens in &spec.ensures {
        let mut stmts = Vec::new();
        let ens_py = translate_expr(world, algs, ens, &mut env, &mut stmts)?;
        for s in stmts {
            py.push_str(&s);
            py.push('\n');
        }
        py.push_str(&format!(
            "if not bool(sp.simplify({ens_py})):\n    raise SystemExit('PROOF=FAIL (ensures)')\n"
        ));
    }

    Ok(())
}

fn translate_expr<'a>(
    world: &World<'a>,
    algs: &HashMap<&'a str, &'a AlgorithmDef>,
    expr: &Expr,
    env: &mut HashMap<String, String>,
    out_stmts: &mut Vec<String>,
) -> Result<String, String> {
    match expr {
        Expr::Number(x) => {
            if !is_int_like(*x) {
                return Err(format!(
                    "proof mode currently only supports integer literals; got {x}. Use --mode run."
                ));
            }
            Ok(format!("sp.Integer({})", *x as i64))
        }
        Expr::Bool(b) => Ok(if *b { "True" } else { "False" }.to_string()),
        Expr::String(s) => Ok(format!("sp.Symbol({:?})", s)),
        Expr::Ident(name) => env
            .get(name)
            .cloned()
            .ok_or_else(|| format!("proof mode: unknown identifier '{name}'")),

        Expr::Unary { op, expr } => {
            let e = translate_expr(world, algs, expr, env, out_stmts)?;
            match op {
                UnOp::Neg => Ok(format!("(-({e}))")),
                UnOp::Not => Ok(format!("sp.Not({e})")),
            }
        }

        Expr::Bin { op, left, right } => {
            let l = translate_expr(world, algs, left, env, out_stmts)?;
            let r = translate_expr(world, algs, right, env, out_stmts)?;
            Ok(match op {
                BinOp::Add => format!("(({l}) + ({r}))"),
                BinOp::Sub => format!("(({l}) - ({r}))"),
                BinOp::Mul => format!("(({l}) * ({r}))"),
                BinOp::Div => format!("(({l}) / ({r}))"),
                BinOp::Pow => format!("(({l}) ** ({r}))"),
                BinOp::Mod => format!("sp.Mod(({l}), ({r}))"),
                BinOp::Eq => format!("sp.Eq(({l}), ({r}))"),
                BinOp::Ne => format!("sp.Ne(({l}), ({r}))"),
                BinOp::Lt => format!("(({l}) < ({r}))"),
                BinOp::Le => format!("(({l}) <= ({r}))"),
                BinOp::Gt => format!("(({l}) > ({r}))"),
                BinOp::Ge => format!("(({l}) >= ({r}))"),
                BinOp::And => format!("sp.And(({l}), ({r}))"),
                BinOp::Or => format!("sp.Or(({l}), ({r}))"),
                BinOp::Implies => format!("sp.Implies(({l}), ({r}))"),
            })
        }

        Expr::Let { name, value, body } => {
            let v = translate_expr(world, algs, value, env, out_stmts)?;
            let py_name = sanitize_ident(name);
            out_stmts.push(format!("{py_name} = {v}"));

            let old = env.insert(name.clone(), py_name);
            let res = translate_expr(world, algs, body, env, out_stmts);
            match old {
                Some(prev) => {
                    env.insert(name.clone(), prev);
                }
                None => {
                    env.remove(name);
                }
            }
            res
        }

        Expr::Set(elems) => {
            let mut xs = Vec::new();
            for e in elems {
                xs.push(translate_expr(world, algs, e, env, out_stmts)?);
            }
            Ok(format!("sp.FiniteSet({})", xs.join(", ")))
        }

        Expr::List(elems) => {
            let mut xs = Vec::new();
            for e in elems {
                xs.push(translate_expr(world, algs, e, env, out_stmts)?);
            }
            Ok(format!("[{}]", xs.join(", ")))
        }

        Expr::Case { arms, default } => {
            let tmp = fresh_tmp(out_stmts);
            // Use a chain of if/elif/else with concrete booleans.
            for (i, (cond, rhs)) in arms.iter().enumerate() {
                let c = translate_expr(world, algs, cond, env, out_stmts)?;
                let r = translate_expr(world, algs, rhs, env, out_stmts)?;
                if i == 0 {
                    out_stmts.push(format!("if bool(sp.simplify({c})):"));
                } else {
                    out_stmts.push(format!("elif bool(sp.simplify({c})):"));
                }
                out_stmts.push(format!("    {tmp} = {r}"));
            }
            let d = translate_expr(world, algs, default, env, out_stmts)?;
            out_stmts.push("else:".to_string());
            out_stmts.push(format!("    {tmp} = {d}"));
            Ok(tmp)
        }

        Expr::Pipe { head, steps } => {
            let mut acc = translate_expr(world, algs, head, env, out_stmts)?;
            for step in steps {
                acc = apply_step(world, algs, step, env, out_stmts, acc)?;
            }
            Ok(acc)
        }

        Expr::Call { is_alg, name, args } => {
            let mut translated_args = Vec::with_capacity(args.len());
            for a in args {
                translated_args.push(translate_expr(world, algs, a, env, out_stmts)?);
            }

            // Resolve name (unqualified -> qualified) using symbol table.
            let resolved = world.symbols.get(name).map(|s| s.as_str()).unwrap_or(name);
            let is_alg_call = *is_alg || algs.contains_key(resolved);

            if is_alg_call {
                let fn_name = py_fn_name(resolved);
                return Ok(format!("{fn_name}({})", translated_args.join(", ")));
            }

            // Builtins: map to SymPy
            let f = match name.as_str() {
                "sqrt" => "sp.sqrt",
                "abs" => "sp.Abs",
                "sin" => "sp.sin",
                "cos" => "sp.cos",
                "tan" => "sp.tan",
                "asin" => "sp.asin",
                "acos" => "sp.acos",
                "atan" => "sp.atan",
                "sinh" => "sp.sinh",
                "cosh" => "sp.cosh",
                "tanh" => "sp.tanh",
                "exp" => "sp.exp",
                "ln" | "log" => "sp.log",
                "log10" => "lambda x: sp.log(x, 10)",
                "log2" => "lambda x: sp.log(x, 2)",
                "floor" => "sp.floor",
                "ceil" => "sp.ceiling",
                "round" => "sp.round",
                "trunc" => "sp.floor",
                "fract" => "lambda x: x - sp.floor(x)",
                "min" => "sp.Min",
                "max" => "sp.Max",
                "atan2" => "sp.atan2",
                "hypot" => "lambda x,y: sp.sqrt(x*x + y*y)",
                "pow" => "lambda x,y: x**y",
                _ => return Err(format!("proof mode: unknown function '{name}'")),
            };

            Ok(format!("({f})({})", translated_args.join(", ")))
        }
    }
}

fn apply_step<'a>(
    world: &World<'a>,
    algs: &HashMap<&'a str, &'a AlgorithmDef>,
    step: &Expr,
    env: &mut HashMap<String, String>,
    out_stmts: &mut Vec<String>,
    input: String,
) -> Result<String, String> {
    match step {
        Expr::Ident(name) => {
            // Treat as a unary call.
            let call = Expr::Call {
                is_alg: false,
                name: name.clone(),
                args: vec![Expr::Ident("__pipe_input".to_string())],
            };
            let old = env.insert("__pipe_input".to_string(), input);
            let res = translate_expr(world, algs, &call, env, out_stmts);
            match old {
                Some(prev) => {
                    env.insert("__pipe_input".to_string(), prev);
                }
                None => {
                    env.remove("__pipe_input");
                }
            }
            res
        }
        Expr::Call { is_alg, name, args } => {
            // Prepend input as first argument.
            let mut new_args = Vec::with_capacity(1 + args.len());
            new_args.push(Expr::Ident("__pipe_input".to_string()));
            new_args.extend(args.iter().cloned());
            let call = Expr::Call {
                is_alg: *is_alg,
                name: name.clone(),
                args: new_args,
            };
            let old = env.insert("__pipe_input".to_string(), input);
            let res = translate_expr(world, algs, &call, env, out_stmts);
            match old {
                Some(prev) => {
                    env.insert("__pipe_input".to_string(), prev);
                }
                None => {
                    env.remove("__pipe_input");
                }
            }
            res
        }
        other => Err(format!(
            "proof mode: pipeline step must be a call or name, got {:?}",
            other
        )),
    }
}

fn fresh_tmp(out_stmts: &[String]) -> String {
    format!("_tmp{}", out_stmts.len())
}

fn py_fn_name(qualified: &str) -> String {
    format!("alg__{}", qualified.replace('.', "__"))
}

fn sanitize_ident(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for (i, ch) in name.chars().enumerate() {
        if (i == 0 && (ch.is_ascii_alphabetic() || ch == '_'))
            || (i > 0 && (ch.is_ascii_alphanumeric() || ch == '_'))
        {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() { "_".to_string() } else { out }
}
