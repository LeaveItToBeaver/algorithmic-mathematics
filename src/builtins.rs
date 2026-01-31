//! Built-in mathematical functions for AM
//!
//! These are the primitive operations that form the foundation of the language.
//! They map directly to standard mathematical notation and Rust's f64 methods.

use crate::eval::Value;

/// A built-in function with its arity and implementation
pub struct Builtin {
    pub name: &'static str,
    pub arity: usize,
    pub func: fn(&[f64]) -> f64,
}

/// All built-in unary functions (arity 1)
const UNARY_BUILTINS: &[(&str, fn(f64) -> f64)] = &[
    // Basic
    ("sqrt", f64::sqrt),
    ("abs", f64::abs),
    ("sign", f64::signum),
    // Trigonometric
    ("sin", f64::sin),
    ("cos", f64::cos),
    ("tan", f64::tan),
    ("asin", f64::asin),
    ("acos", f64::acos),
    ("atan", f64::atan),
    ("sinh", f64::sinh),
    ("cosh", f64::cosh),
    ("tanh", f64::tanh),
    // Exponential & Logarithmic
    ("exp", f64::exp),
    ("ln", f64::ln),  // natural log
    ("log", f64::ln), // alias for ln (math convention)
    ("log10", f64::log10),
    ("log2", f64::log2),
    // Rounding
    ("floor", f64::floor),
    ("ceil", f64::ceil),
    ("round", f64::round),
    ("trunc", f64::trunc),
    ("fract", f64::fract),
];

/// All built-in binary functions (arity 2)
const BINARY_BUILTINS: &[(&str, fn(f64, f64) -> f64)] = &[
    ("min", f64::min),
    ("max", f64::max),
    ("atan2", f64::atan2),
    ("hypot", f64::hypot), // sqrt(x² + y²)
    ("pow", f64::powf),    // x^y (also available as ^ operator)
];

/// Look up a builtin function and call it with the given values
pub fn call_builtin(name: &str, vals: &[Value]) -> Result<Value, String> {
    // Check unary builtins
    for (builtin_name, func) in UNARY_BUILTINS {
        if name == *builtin_name {
            check_arity(name, vals.len(), 1)?;
            let x = vals[0].as_f64()?;
            return Ok(Value::Number(func(x)));
        }
    }

    // Check binary builtins
    for (builtin_name, func) in BINARY_BUILTINS {
        if name == *builtin_name {
            check_arity(name, vals.len(), 2)?;
            let x = vals[0].as_f64()?;
            let y = vals[1].as_f64()?;
            return Ok(Value::Number(func(x, y)));
        }
    }

    Err(format!("unknown function: {}", name))
}

/// Check if a name is a builtin function
pub fn is_builtin(name: &str) -> bool {
    UNARY_BUILTINS.iter().any(|(n, _)| *n == name)
        || BINARY_BUILTINS.iter().any(|(n, _)| *n == name)
}

/// Get all builtin function names (for help/autocomplete)
pub fn builtin_names() -> Vec<&'static str> {
    let mut names: Vec<&str> = Vec::new();
    names.extend(UNARY_BUILTINS.iter().map(|(n, _)| *n));
    names.extend(BINARY_BUILTINS.iter().map(|(n, _)| *n));
    names
}

fn check_arity(name: &str, got: usize, expected: usize) -> Result<(), String> {
    if got != expected {
        Err(format!("{} expects {} arg(s), got {}", name, expected, got))
    } else {
        Ok(())
    }
}
