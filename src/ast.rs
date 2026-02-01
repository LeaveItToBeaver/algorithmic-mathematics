#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Bool(bool),
    String(String),
    Ident(String),
    Call {
        is_alg: bool,
        name: String,
        args: Vec<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
    Bin {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Case {
        arms: Vec<(Expr, Expr)>,
        default: Box<Expr>,
    },
    Pipe {
        head: Box<Expr>,
        steps: Vec<Expr>,
    },
    /// Let binding: `let x = expr in body`
    Let {
        name: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    /// Set literal: `{1, 2, 3}`
    Set(Vec<Expr>),
    /// List literal: `[1, 2, 3]` - but we use `list(1, 2, 3)` since [] is for case
    List(Vec<Expr>),
    
    // === NEW: Lambdas ===
    /// Lambda expression: `\x -> x + 1` or `\x, y -> x + y`
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    /// Apply a lambda or function value to arguments
    Apply {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    
    // === NEW: ADT Constructors ===
    /// Constructor application: `Some(5)`, `Cons(1, Nil)`
    Constructor {
        name: String,
        args: Vec<Expr>,
    },
    
    // === NEW: Pattern Matching ===
    /// Match expression: `match e with | Pat1 -> e1 | Pat2 -> e2 end`
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    
    // === NEW: IO Operations ===
    /// Print expression (returns Unit, side effect)
    Print(Box<Expr>),
    /// Read file contents
    ReadFile(Box<Expr>),
    /// Write to file
    WriteFile {
        path: Box<Expr>,
        content: Box<Expr>,
    },
}

/// A single arm in a match expression
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

/// Patterns for matching
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Wildcard: `_`
    Wildcard,
    /// Variable binding: `x`
    Var(String),
    /// Literal number: `0`, `42`
    Number(f64),
    /// Literal bool: `true`, `false`
    Bool(bool),
    /// Literal string: `"hello"`
    String(String),
    /// Constructor pattern: `Some(x)`, `Cons(h, t)`
    Constructor {
        name: String,
        args: Vec<Pattern>,
    },
    /// Tuple pattern: `(x, y)`
    Tuple(Vec<Pattern>),
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Implies,
}

/// === Type System ===

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Basic types
    Real,         // R or ℝ
    Natural,      // N or ℕ
    Integer,      // Z or ℤ
    Rational,     // Q or ℚ
    Complex,      // C or ℂ
    Bool,         // Bool
    Unit,         // Unit / () - for IO operations that return nothing

    /// Parameterized types
    Set(Box<Type>),              // Set<T>
    Vec(Box<Type>, Option<u32>), // Vec<T, n> or Vec<T>
    List(Box<Type>),             // List<T>
    Option(Box<Type>),           // Option<T>
    Tuple(Vec<Type>),            // (T1, T2, ...)
    
    /// Function type
    Function {
        params: Vec<Type>,
        result: Box<Type>,
    },
    
    /// IO wrapper type
    IO(Box<Type>),               // IO<T> - effectful computation

    /// Vector space with dimension
    VecSpace(Box<Type>, u32), // R^3

    /// Named/user-defined types
    Named(String),
    
    /// Type variable for generics
    TypeVar(String),            // T, U, etc.
}

/// Algebraic Data Type definition
/// `type Option<T> = None | Some(T)`
#[derive(Debug, Clone)]
pub struct TypeDef {
    pub name: String,
    pub type_params: Vec<String>,    // e.g., ["T"] for Option<T>
    pub variants: Vec<Variant>,
}

/// A variant in an ADT
/// `Some(T)` or `None`
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Type>,           // empty for nullary constructors like None
}

/// Optional specification for algorithms
#[derive(Debug, Clone)]
pub struct Specification {
    pub requires: Vec<Expr>, // Preconditions
    pub ensures: Vec<Expr>,  // Postconditions
}

#[derive(Debug, Clone)]
pub struct AlgorithmDef {
    pub name: String,
    pub params: Vec<(String, Option<Type>)>, // Changed from Vec<String>
    pub return_type: Option<Type>,            // NEW
    pub spec: Option<Specification>,          // NEW
    pub body: Expr,
}

/// === Modules ===

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub imports: Vec<Import>,
    pub uses: Vec<UseItem>,
    pub exports: Vec<String>,
    pub reexports: Vec<ReexportItem>,
    pub decls: Vec<TopLevelDecl>,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: ModPath,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub enum UseItem {
    Star { path: ModPath },
    Named { path: ModPath, ident: String },
}

#[derive(Debug, Clone)]
pub struct ReexportItem {
    pub path: ModPath,
    pub ident: String,
}

#[derive(Debug, Clone)]
pub struct ModPath {
    pub segments: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum TopLevelDecl {
    Definition(AlgorithmDef),
    TypeDef(TypeDef),    // NEW: ADT definitions
    Include(String),     // path
}

pub fn show_expr(e: &Expr, indent: usize) {
    let pad = "  ".repeat(indent);
    match e {
        Expr::Number(v) => println!("{pad}Number({v})"),
        Expr::Bool(b) => println!("{pad}Bool({b})"),
        Expr::String(s) => println!("{pad}String(\"{s}\")"),
        Expr::Ident(s) => println!("{pad}Ident({s})"),
        Expr::Call { is_alg, name, args } => {
            println!("{pad}Call is_alg={is_alg} name={name}");
            for a in args {
                show_expr(a, indent + 1);
            }
        }
        Expr::Unary { op, expr } => {
            println!("{pad}Unary({:?})", op);
            show_expr(expr, indent + 1);
        }
        Expr::Bin { op, left, right } => {
            println!("{pad}Bin({:?})", op);
            show_expr(left, indent + 1);
            show_expr(right, indent + 1);
        }
        Expr::Case { arms, default } => {
            println!("{pad}Case");
            for (c, v) in arms {
                show_expr(c, indent + 1);
                show_expr(v, indent + 1);
            }
            println!("{pad}Default:");
            show_expr(default, indent + 1);
        }
        Expr::Pipe { head, steps } => {
            println!("{pad}Pipe");
            show_expr(head, indent + 1);
            for s in steps {
                show_expr(s, indent + 1);
            }
        }
        Expr::Let { name, value, body } => {
            println!("{pad}Let {name} =");
            show_expr(value, indent + 1);
            println!("{pad}in");
            show_expr(body, indent + 1);
        }
        Expr::Set(elems) => {
            println!("{pad}Set {{");
            for e in elems {
                show_expr(e, indent + 1);
            }
            println!("{pad}}}");
        }
        Expr::List(elems) => {
            println!("{pad}List [");
            for e in elems {
                show_expr(e, indent + 1);
            }
            println!("{pad}]");
        }
        Expr::Lambda { params, body } => {
            println!("{pad}Lambda({}) ->", params.join(", "));
            show_expr(body, indent + 1);
        }
        Expr::Apply { func, args } => {
            println!("{pad}Apply");
            show_expr(func, indent + 1);
            for a in args {
                show_expr(a, indent + 1);
            }
        }
        Expr::Constructor { name, args } => {
            println!("{pad}Constructor {name}");
            for a in args {
                show_expr(a, indent + 1);
            }
        }
        Expr::Match { scrutinee, arms } => {
            println!("{pad}Match");
            show_expr(scrutinee, indent + 1);
            for arm in arms {
                println!("{pad}  | {:?} ->", arm.pattern);
                show_expr(&arm.body, indent + 2);
            }
        }
        Expr::Print(e) => {
            println!("{pad}Print");
            show_expr(e, indent + 1);
        }
        Expr::ReadFile(path) => {
            println!("{pad}ReadFile");
            show_expr(path, indent + 1);
        }
        Expr::WriteFile { path, content } => {
            println!("{pad}WriteFile");
            show_expr(path, indent + 1);
            show_expr(content, indent + 1);
        }
    }
}

/// Display type in ASCII form
pub fn show_type(ty: &Type) -> String {
    match ty {
        Type::Real => "R".to_string(),
        Type::Natural => "N".to_string(),
        Type::Integer => "Z".to_string(),
        Type::Rational => "Q".to_string(),
        Type::Complex => "C".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::Unit => "Unit".to_string(),
        Type::Set(inner) => format!("Set<{}>", show_type(inner)),
        Type::Vec(inner, Some(n)) => format!("Vec<{}, {}>", show_type(inner), n),
        Type::Vec(inner, None) => format!("Vec<{}>", show_type(inner)),
        Type::List(inner) => format!("List<{}>", show_type(inner)),
        Type::Option(inner) => format!("Option<{}>", show_type(inner)),
        Type::Tuple(types) => {
            let types_str = types
                .iter()
                .map(show_type)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", types_str)
        }
        Type::Function { params, result } => {
            let params_str = params
                .iter()
                .map(show_type)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({}) -> {}", params_str, show_type(result))
        }
        Type::IO(inner) => format!("IO<{}>", show_type(inner)),
        Type::VecSpace(base, dim) => format!("{}^{}", show_type(base), dim),
        Type::Named(name) => name.clone(),
        Type::TypeVar(name) => name.clone(),
    }
}

/// Format parameter list for display
pub fn show_params(params: &[(String, Option<Type>)]) -> String {
    params
        .iter()
        .map(|(name, ty)| match ty {
            Some(t) => format!("{}: {}", name, show_type(t)),
            None => name.clone(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
