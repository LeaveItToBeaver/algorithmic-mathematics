#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Bool(bool),
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

    /// Parameterized types
    Set(Box<Type>),              // Set<T>
    Vec(Box<Type>, Option<u32>), // Vec<T, n> or Vec<T>
    List(Box<Type>),             // List<T>
    Option(Box<Type>),           // Option<T>
    Tuple(Vec<Type>),            // (T1, T2, ...)

    /// Vector space with dimension
    VecSpace(Box<Type>, u32), // R^3

    /// Named/user-defined types
    Named(String),
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
    Include(String), // path
                     // Structure/Instance/etc can be added later
}

pub fn show_expr(e: &Expr, indent: usize) {
    let pad = "  ".repeat(indent);
    match e {
        Expr::Number(v) => println!("{pad}Number({v})"),
        Expr::Bool(b) => println!("{pad}Bool({b})"),
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
        Type::VecSpace(base, dim) => format!("{}^{}", show_type(base), dim),
        Type::Named(name) => name.clone(),
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
