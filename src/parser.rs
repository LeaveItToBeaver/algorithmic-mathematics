use crate::ast::{AlgorithmDef, BinOp, Expr, UnOp};
use crate::token::{TokSpan, Token, caret_message};

pub struct Tokens<'a> {
        items: Vec<TokSpan>,
        pos: usize,
        src: &'a str, // NEW: keep the source for caret messages
}

impl<'a> Tokens<'a> {
        pub fn new_with_src(items: Vec<TokSpan>, src: &'a str) -> Self {
                Self { items, pos: 0, src }
        }
        pub fn peek(&self) -> Option<&Token> {
                self.items.get(self.pos).map(|t| &t.tok)
        }
        fn peek_span(&self) -> Option<&TokSpan> {
                self.items.get(self.pos)
        }
        fn last_span(&self) -> Option<&TokSpan> {
                if self.pos == 0 {
                        None
                } else {
                        self.items.get(self.pos - 1)
                }
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
                        let byte = self
                                .peek_span()
                                .map(|s| s.start)
                                .or_else(|| self.last_span().map(|s| s.end))
                                .unwrap_or(0);
                        let msg = format!("expected {:?} while parsing {}", want, ctx);
                        let pretty = caret_message(self.src, byte, &msg);
                        panic!("{}", pretty);
                }
        }

        fn err_here<T>(&self, msg: &str) -> T {
                let byte = self
                        .peek_span()
                        .map(|s| s.start)
                        .or_else(|| self.last_span().map(|s| s.end))
                        .unwrap_or(0);
                let pretty = caret_message(self.src, byte, msg);
                panic!("{}", pretty);
        }
}

/* AlgDef := '@' Ident '(' [Ident {',' Ident}] ')' '=' Expr */
pub fn parse_alg_def(ts: &mut Tokens) -> AlgorithmDef {
        ts.expect(&Token::At, "algorithm start '@'");
        let name = parse_algorithm_name(ts);
        ts.expect(&Token::LParen, "parameter list '('");
        let params = parse_parameter_list(ts);
        ts.expect(&Token::RParen, "parameter list ')'");
        ts.expect(&Token::Equal, "definition '='");
        let body = parse_expr(ts);
        AlgorithmDef { name, params, body }
}

fn parse_algorithm_name(ts: &mut Tokens) -> String {
        match ts.next() {
                Some(Token::Ident(s)) => s,
                other => ts.err_here(&format!("expected identifier after '@', got {:?}", other)),
        }
}

fn parse_parameter_list(ts: &mut Tokens) -> Vec<String> {
        let mut params = Vec::new();

        while let Some(Token::Ident(_)) = ts.peek() {
                if let Some(Token::Ident(s)) = ts.next() {
                        params.push(s);
                }

                if !ts.eat(&Token::Comma) {
                        break;
                }
        }

        params
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
                        default = Some(parse_default_arm(ts));
                } else {
                        parse_conditional_arm(ts, &mut arms);
                }

                if !ts.eat(&Token::Semicolon) {
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

fn parse_default_arm(ts: &mut Tokens) -> Expr {
        if ts.eat(&Token::QMark) || ts.eat(&Token::Arrow) {
                parse_expr(ts)
        } else {
                ts.err_here("expected '?' or '->' after '_' in case arm")
        }
}

fn parse_conditional_arm(ts: &mut Tokens, arms: &mut Vec<(Expr, Expr)>) {
        let cond = parse_or(ts);

        if ts.eat(&Token::QMark) {
                parse_question_arm(ts, arms, cond);
        } else if ts.eat(&Token::Arrow) {
                let rhs = parse_expr(ts);
                arms.push((cond, rhs));
        } else {
                ts.err_here::<()>("expected '?' or '->' after condition in case arm");
        }
}

fn parse_question_arm(ts: &mut Tokens, arms: &mut Vec<(Expr, Expr)>, cond: Expr) {
        let then_e = parse_expr(ts);

        if ts.eat(&Token::Pipe) {
                // cond ? then | else  desugars into two arms
                let else_e = parse_expr(ts);
                arms.push((cond.clone(), then_e));
                let not_cond = Expr::Unary {
                        op: UnOp::Not,
                        expr: Box::new(cond),
                };
                arms.push((not_cond, else_e));
        } else {
                arms.push((cond, then_e));
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
        parse_binary_left_associative(ts, parse_and, &[(Token::DblPipe, BinOp::Or)])
}

fn parse_and(ts: &mut Tokens) -> Expr {
        parse_binary_left_associative(ts, parse_cmp, &[(Token::DblAmp, BinOp::And)])
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

fn make_binary_expr(op: BinOp, left: Expr, right: Expr) -> Expr {
        Expr::Bin {
                op,
                left: Box::new(left),
                right: Box::new(right),
        }
}

fn parse_binary_left_associative<F>(
        ts: &mut Tokens,
        next_level: F,
        operators: &[(Token, BinOp)],
) -> Expr
where
        F: Fn(&mut Tokens) -> Expr,
{
        let mut node = next_level(ts);
        loop {
                let found_op = operators.iter().find(|(token, _)| ts.peek() == Some(token));

                if let Some((_, op)) = found_op {
                        ts.next(); // consume operator
                        let rhs = next_level(ts);
                        node = make_binary_expr(*op, node, rhs);
                } else {
                        break;
                }
        }
        node
}

fn parse_add(ts: &mut Tokens) -> Expr {
        parse_binary_left_associative(
                ts,
                parse_mul,
                &[(Token::Plus, BinOp::Add), (Token::Minus, BinOp::Sub)],
        )
}

fn parse_mul(ts: &mut Tokens) -> Expr {
        parse_binary_left_associative(
                ts,
                parse_pow,
                &[
                        (Token::Star, BinOp::Mul),
                        (Token::Slash, BinOp::Div),
                        (Token::Percent, BinOp::Mod),
                ],
        )
}

fn parse_pow(ts: &mut Tokens) -> Expr {
        let mut node = parse_unary(ts);
        if let Some(Token::Caret) = ts.peek() {
                ts.next();
                let rhs = parse_pow(ts);
                node = Expr::Bin {
                        op: BinOp::Pow,
                        left: Box::new(node),
                        right: Box::new(rhs),
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
        let mut node = parse_primary(ts);
        parse_function_calls(ts, &mut node);
        node
}

fn parse_primary(ts: &mut Tokens) -> Expr {
        match ts.next() {
                Some(Token::Number(s)) => parse_number(ts, &s),
                Some(Token::Bool(b)) => Expr::Bool(b),
                Some(Token::Ident(s)) => Expr::Ident(s),
                Some(Token::At) => parse_algorithm_call(ts),
                Some(Token::LParen) => parse_parenthesized(ts),
                other => ts.err_here(&format!("unexpected token in expression: {:?}", other)),
        }
}

fn parse_number(ts: &mut Tokens, s: &str) -> Expr {
        let v: f64 = s
                .parse()
                .unwrap_or_else(|_| ts.err_here(&format!("bad number literal: {}", s)));
        Expr::Number(v)
}

fn parse_algorithm_call(ts: &mut Tokens) -> Expr {
        let name = match ts.next() {
                Some(Token::Ident(s)) => s,
                other => ts.err_here(&format!("expected identifier after '@', got {:?}", other)),
        };
        Expr::Call {
                is_alg: true,
                name,
                args: Vec::new(),
        }
}

fn parse_parenthesized(ts: &mut Tokens) -> Expr {
        let e = parse_expr(ts);
        match ts.next() {
                Some(Token::RParen) => e,
                other => ts.err_here(&format!("expected ')', got {:?}", other)),
        }
}

fn parse_function_calls(ts: &mut Tokens, node: &mut Expr) {
        while let Some(Token::LParen) = ts.peek() {
                ts.next(); // consume '('
                let args = parse_argument_list(ts);
                ts.expect(&Token::RParen, "closing ')' of call");
                *node = attach_call_to_node(ts, std::mem::replace(node, Expr::Bool(false)), args);
        }
}

fn parse_argument_list(ts: &mut Tokens) -> Vec<Expr> {
        let mut args = Vec::new();

        if let Some(t) = ts.peek() {
                if t != &Token::RParen {
                        args.push(parse_expr(ts));
                        while let Some(Token::Comma) = ts.peek() {
                                ts.next();
                                args.push(parse_expr(ts));
                        }
                }
        }

        args
}

fn attach_call_to_node(ts: &mut Tokens, node: Expr, args: Vec<Expr>) -> Expr {
        match node {
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
                other => ts.err_here(&format!("cannot call non-name expression: {:?}", other)),
        }
}
