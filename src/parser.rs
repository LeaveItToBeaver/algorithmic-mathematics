// src/parser.rs
use crate::ast::{AlgorithmDef, BinOp, Expr, UnOp};
use crate::token::{TokSpan, Token};

pub struct Tokens {
    items: Vec<TokSpan>,
    pos: usize,
}

impl Tokens {
    pub fn new(items: Vec<TokSpan>) -> Self {
        Self { items, pos: 0 }
    }
    pub fn peek(&self) -> Option<&Token> {
        self.items.get(self.pos).map(|t| &t.tok)
    }
    fn next(&mut self) -> Option<Token> {
        if self.pos >= self.items.len() {
            None
        } else {
            let t = self.items[self.pos].tok.clone();
            self.pos += 1;
            Some(t)
        }
    }
    fn eat(&mut self, want: &Token) -> bool {
        if let Some(t) = self.peek() {
            if t == want {
                self.pos += 1;
                return true;
            }
        }
        false
    }
    fn expect(&mut self, want: &Token, ctx: &str) {
        if !self.eat(want) {
            panic!("expected {:?} while parsing {}", want, ctx);
        }
    }
}

/* AlgDef := '@' Ident '(' [Ident {',' Ident}] ')' '=' Expr */
pub fn parse_alg_def(ts: &mut Tokens) -> AlgorithmDef {
    ts.expect(&Token::At, "algorithm start '@'");
    let name = match ts.next() {
        Some(Token::Ident(s)) => s,
        other => panic!("expected identifier after '@', got {:?}", other),
    };
    ts.expect(&Token::LParen, "parameter list '('");
    let mut params = Vec::new();
    loop {
        match ts.peek() {
            Some(Token::Ident(_)) => {
                if let Some(Token::Ident(s)) = ts.next() {
                    params.push(s);
                }
                if ts.eat(&Token::Comma) {
                    continue;
                }
            }
            _ => {}
        }
        break;
    }
    ts.expect(&Token::RParen, "parameter list ')'");
    ts.expect(&Token::Equal, "definition '='");
    let body = parse_expr(ts);
    AlgorithmDef { name, params, body }
}

/* Expr := Case | Pipe
   Pipe := Or { '>>' Or }       // left-assoc into Expr::Pipe
   Case := '[' Arm {';' Arm} ']'   Arm := Cond '?' Expr | '_' '?' Expr
*/
pub fn parse_expr(ts: &mut Tokens) -> Expr {
    // Case has the lowest precedence; check for it explicitly
    if let Some(Token::LBracket) = ts.peek() {
        return parse_case(ts);
    }
    parse_pipe(ts)
}

fn parse_case(ts: &mut Tokens) -> Expr {
    ts.expect(&Token::LBracket, "case '['");
    let mut arms: Vec<(Expr, Expr)> = Vec::new();
    let mut default: Option<Expr> = None;

    loop {
        if ts.eat(&Token::Underscore) {
            // default: _ ? expr   OR   _ -> expr
            if ts.eat(&Token::QMark) || ts.eat(&Token::Arrow) {
                let rhs = parse_expr(ts);
                default = Some(rhs);
            } else {
                panic!("expected '?' or '->' after '_' in case arm");
            }
        } else {
            // cond ? expr [ '|' expr ]   OR   cond -> expr
            let cond = parse_or(ts);

            if ts.eat(&Token::QMark) {
                let then_e = parse_expr(ts);
                if ts.eat(&Token::Pipe) {
                    // cond ? then | else  desugars into two arms + we must continue
                    let else_e = parse_expr(ts);
                    arms.push((cond.clone(), then_e));
                    // else arm becomes (!cond -> else_e)
                    let not_cond = Expr::Unary {
                        op: UnOp::Not,
                        expr: Box::new(cond),
                    };
                    arms.push((not_cond, else_e));
                } else {
                    arms.push((cond, then_e));
                }
            } else if ts.eat(&Token::Arrow) {
                let rhs = parse_expr(ts);
                arms.push((cond, rhs));
            } else {
                panic!("expected '?' or '->' after condition in case arm");
            }
        }

        if ts.eat(&Token::Semicolon) {
            continue;
        } else {
            break;
        }
    }

    ts.expect(&Token::RBracket, "closing ']'");
    let def = default.expect("case block missing default '_' ? expr");
    Expr::Case {
        arms,
        default: Box::new(def),
    }
}

fn parse_pipe(ts: &mut Tokens) -> Expr {
    let head = parse_or(ts);
    let mut steps: Vec<Expr> = Vec::new();
    while ts.eat(&Token::DblGt) {
        let step = parse_or(ts);
        steps.push(step);
    }
    if steps.is_empty() {
        head
    } else {
        Expr::Pipe {
            head: Box::new(head),
            steps,
        }
    }
}

/* precedence ladder: Or → And → Cmp → Add → Mul → Unary → Postfix → Primary
   Postfix here adds function calls after a primary:  name(args)  or  @Name(args)
*/

fn parse_or(ts: &mut Tokens) -> Expr {
    let mut node = parse_and(ts);
    while ts.eat(&Token::DblPipe) {
        let rhs = parse_and(ts);
        node = Expr::Bin {
            op: BinOp::Or,
            left: Box::new(node),
            right: Box::new(rhs),
        };
    }
    node
}

fn parse_and(ts: &mut Tokens) -> Expr {
    let mut node = parse_cmp(ts);
    while ts.eat(&Token::DblAmp) {
        let rhs = parse_cmp(ts);
        node = Expr::Bin {
            op: BinOp::And,
            left: Box::new(node),
            right: Box::new(rhs),
        };
    }
    node
}

fn parse_cmp(ts: &mut Tokens) -> Expr {
    let mut node = parse_add(ts);
    let op = match ts.peek() {
        Some(Token::EqEq) | Some(Token::Equal) => Some(BinOp::Eq), // accept '=' as equality too
        Some(Token::Neq) => Some(BinOp::Ne),
        Some(Token::Le) => Some(BinOp::Le),
        Some(Token::Ge) => Some(BinOp::Ge),
        Some(Token::Lt) => Some(BinOp::Lt),
        Some(Token::Gt) => Some(BinOp::Gt),
        _ => None,
    };
    if let Some(op) = op {
        ts.next();
        let rhs = parse_add(ts);
        node = Expr::Bin {
            op,
            left: Box::new(node),
            right: Box::new(rhs),
        };
    }
    node
}

fn parse_add(ts: &mut Tokens) -> Expr {
    let mut node = parse_mul(ts);
    loop {
        match ts.peek() {
            Some(Token::Plus) => {
                ts.next();
                let rhs = parse_mul(ts);
                node = Expr::Bin {
                    op: BinOp::Add,
                    left: Box::new(node),
                    right: Box::new(rhs),
                };
            }
            Some(Token::Minus) => {
                ts.next();
                let rhs = parse_mul(ts);
                node = Expr::Bin {
                    op: BinOp::Sub,
                    left: Box::new(node),
                    right: Box::new(rhs),
                };
            }
            _ => break,
        }
    }
    node
}

fn parse_mul(ts: &mut Tokens) -> Expr {
    let mut node = parse_unary(ts);
    loop {
        match ts.peek() {
            Some(Token::Star) => {
                ts.next();
                let rhs = parse_unary(ts);
                node = Expr::Bin {
                    op: BinOp::Mul,
                    left: Box::new(node),
                    right: Box::new(rhs),
                };
            }
            Some(Token::Slash) => {
                ts.next();
                let rhs = parse_unary(ts);
                node = Expr::Bin {
                    op: BinOp::Div,
                    left: Box::new(node),
                    right: Box::new(rhs),
                };
            }
            _ => break,
        }
    }
    node
}

fn parse_unary(ts: &mut Tokens) -> Expr {
    if ts.eat(&Token::Minus) {
        let e = parse_unary(ts);
        Expr::Unary {
            op: UnOp::Neg,
            expr: Box::new(e),
        }
    } else if ts.eat(&Token::Bang) {
        let e = parse_unary(ts);
        Expr::Unary {
            op: UnOp::Not,
            expr: Box::new(e),
        }
    } else {
        parse_postfix(ts)
    }
}

fn parse_postfix(ts: &mut Tokens) -> Expr {
    // Primary
    let mut node = match ts.next() {
        Some(Token::Number(s)) => {
            let v: f64 = s
                .parse()
                .unwrap_or_else(|_| panic!("bad number literal: {}", s));
            Expr::Number(v)
        }
        Some(Token::Bool(b)) => Expr::Bool(b),
        Some(Token::Ident(s)) => Expr::Ident(s),
        Some(Token::At) => {
            // @Name must be followed by ident
            let name = match ts.next() {
                Some(Token::Ident(s)) => s,
                other => panic!("expected identifier after '@', got {:?}", other),
            };
            // If followed by '(', this will be handled below as a call
            Expr::Call {
                is_alg: true,
                name,
                args: Vec::new(),
            }
        }
        Some(Token::LParen) => {
            let e = parse_expr(ts);
            match ts.next() {
                Some(Token::RParen) => e,
                other => panic!("expected ')', got {:?}", other),
            }
        }
        other => panic!("unexpected token in expression: {:?}", other),
    };

    // Postfix: function calls after names or @Name
    loop {
        match ts.peek() {
            Some(Token::LParen) => {
                // consume '('
                ts.next();
                let mut args = Vec::new();
                // optional args
                if let Some(t) = ts.peek() {
                    if t != &Token::RParen {
                        args.push(parse_expr(ts));
                        while let Some(Token::Comma) = ts.peek() {
                            ts.next();
                            args.push(parse_expr(ts));
                        }
                    }
                }
                // close ')'
                ts.expect(&Token::RParen, "closing ')' of call");

                // attach call to current node
                node = match node {
                    Expr::Ident(name) => Expr::Call {
                        is_alg: false,
                        name,
                        args,
                    },
                    Expr::Call {
                        is_alg: true, name, ..
                    } => Expr::Call {
                        is_alg: true,
                        name,
                        args,
                    },
                    other => panic!("cannot call non-name expression: {:?}", other),
                };
            }
            _ => break,
        }
    }

    node
}
