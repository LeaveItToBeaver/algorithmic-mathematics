#[derive(Debug, Clone)]
pub enum Expr {
        Number(f64),
        Bool(bool),
        Ident(String),
        Call {
                is_alg: bool,
                name: String,
                args: Vec<Expr>,
        }, // f(x) or @Alg(x)
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
        }, // x >> @f >> g
}

#[derive(Debug, Copy, Clone)]
pub enum UnOp {
        Neg,
        Not,
}

#[derive(Debug, Copy, Clone)]
pub enum BinOp {
        Add,
        Sub,
        Mul,
        Div,
        Pow,
        Mod,
        Eq,
        Ne,
        Lt,
        Le,
        Gt,
        Ge,
        And,
        Or,
}

#[derive(Debug)]
pub struct AlgorithmDef {
        pub name: String,
        pub params: Vec<String>,
        pub body: Expr,
}

pub fn show_expr(e: &Expr, indent: usize) {
        let pad = "  ".repeat(indent);
        match e {
                Expr::Number(v) => println!("{pad}Number({v})"),
                Expr::Bool(b) => println!("{pad}Bool({b})"),
                Expr::Ident(s) => println!("{pad}Ident({s})"),
                Expr::Call { is_alg, name, args } => {
                        println!("{pad}Call(is_alg={is_alg}, name={name})");
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
                        for (c, r) in arms {
                                println!("{pad}  Arm:");
                                show_expr(c, indent + 2);
                                println!("{pad}  =>");
                                show_expr(r, indent + 2);
                        }
                        println!("{pad}  Default:");
                        show_expr(default, indent + 2);
                }
                Expr::Pipe { head, steps } => {
                        println!("{pad}Pipe");
                        println!("{pad}  Head:");
                        show_expr(head, indent + 2);
                        for s in steps {
                                println!("{pad}  >> Step:");
                                show_expr(s, indent + 2);
                        }
                }
        }
}
