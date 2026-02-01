use crate::ast::{
    AlgorithmDef, BinOp, Expr, MatchArm, Pattern, Specification, Type, TypeDef, UnOp, Variant,
};
use crate::ast::{Import, ModPath, Module, ReexportItem, TopLevelDecl, UseItem};
use crate::token::{TokSpan, Token, caret_message};

pub struct Tokens<'a> {
    items: Vec<TokSpan>,
    pos: usize,
    src: &'a str,
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
                .map(|s| s.span.start)
                .or_else(|| self.last_span().map(|s| s.span.end))
                .unwrap_or(0);
            let msg = format!("expected {:?} while parsing {}", want, ctx);
            let pretty = caret_message(self.src, byte, &msg);
            panic!("{}", pretty);
        }
    }

    fn err_here(&self, msg: &str) -> ! {
        let byte = self
            .peek_span()
            .map(|s| s.span.start)
            .or_else(|| self.last_span().map(|s| s.span.end))
            .unwrap_or(0);
        let pretty = caret_message(self.src, byte, msg);
        panic!("{}", pretty);
    }
    fn expect_ident(&mut self, ctx: &str) -> String {
        match self.next() {
            Some(Token::Ident(s)) => s,
            other => self.err_here(&format!(
                "expected identifier while parsing {ctx}, got {:?}",
                other
            )),
        }
    }

    fn expect_string(&mut self, ctx: &str) -> String {
        match self.next() {
            Some(Token::String(s)) => s,
            other => self.err_here(&format!(
                "expected string while parsing {ctx}, got {:?}",
                other
            )),
        }
    }
}

/* AlgDef := '@' Ident '(' [TypedParam {',' TypedParam}] ')' ['->' Type] [Spec] ':=' Expr
   where TypedParam := Ident [':' Type]
*/
pub fn parse_alg_def(ts: &mut Tokens) -> AlgorithmDef {
    ts.expect(&Token::At, "algorithm start '@'");
    let name = parse_algorithm_name(ts);
    ts.expect(&Token::LParen, "parameter list '('");
    let params = parse_typed_parameter_list(ts);
    ts.expect(&Token::RParen, "parameter list ')'");

    // Optional return type: -> Type
    let return_type = if ts.eat(&Token::Arrow) {
        Some(parse_type(ts))
    } else {
        None
    };

    // Optional specification (requires/ensures)
    let spec = if matches!(ts.peek(), Some(Token::KwRequires)) {
        Some(parse_specification(ts))
    } else {
        None
    };

    // Definition operator: := or = (for backwards compatibility)
    if !ts.eat(&Token::Defines) {
        ts.expect(&Token::Equal, "definition ':=' or '='");
    }

    let body = parse_expr(ts);

    AlgorithmDef {
        name,
        params,
        return_type,
        spec,
        body,
    }
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

/// Parse parameter list with optional type annotations: name or name: Type
fn parse_typed_parameter_list(ts: &mut Tokens) -> Vec<(String, Option<Type>)> {
    let mut params = Vec::new();

    while let Some(Token::Ident(_)) = ts.peek() {
        if let Some(Token::Ident(s)) = ts.next() {
            let ty = if ts.eat(&Token::Colon) {
                Some(parse_type(ts))
            } else {
                None
            };
            params.push((s, ty));
        }

        if !ts.eat(&Token::Comma) {
            break;
        }
    }

    params
}

/// Parse a type annotation
fn parse_type(ts: &mut Tokens) -> Type {
    match ts.peek() {
        Some(Token::Ident(name)) => {
            let name = name.clone();
            ts.next();

            // Check for basic types
            let base_type = match name.as_str() {
                "R" => Type::Real,
                "N" => Type::Natural,
                "Z" => Type::Integer,
                "Q" => Type::Rational,
                "C" => Type::Complex,
                "Bool" => Type::Bool,
                "Set" => {
                    // Set<Type>
                    ts.expect(&Token::Lt, "generic type parameter '<'");
                    let inner = parse_type(ts);
                    ts.expect(&Token::Gt, "generic type parameter '>'");
                    return Type::Set(Box::new(inner));
                }
                "Vec" => {
                    // Vec<Type> or Vec<Type, n>
                    ts.expect(&Token::Lt, "generic type parameter '<'");
                    let inner = parse_type(ts);
                    let dim = if ts.eat(&Token::Comma) {
                        match ts.next() {
                            Some(Token::Number(n)) => Some(n.parse::<u32>().unwrap_or(0)),
                            _ => ts.err_here("expected number for vector dimension"),
                        }
                    } else {
                        None
                    };
                    ts.expect(&Token::Gt, "generic type parameter '>'");
                    return Type::Vec(Box::new(inner), dim);
                }
                "List" => {
                    // List<Type>
                    ts.expect(&Token::Lt, "generic type parameter '<'");
                    let inner = parse_type(ts);
                    ts.expect(&Token::Gt, "generic type parameter '>'");
                    return Type::List(Box::new(inner));
                }
                "Option" => {
                    // Option<Type>
                    ts.expect(&Token::Lt, "generic type parameter '<'");
                    let inner = parse_type(ts);
                    ts.expect(&Token::Gt, "generic type parameter '>'");
                    return Type::Option(Box::new(inner));
                }
                _ => Type::Named(name),
            };

            // Check for superscript (R^3)
            if ts.eat(&Token::Caret) {
                match ts.next() {
                    Some(Token::Number(n)) => {
                        let dim = n.parse::<u32>().unwrap_or(0);
                        Type::VecSpace(Box::new(base_type), dim)
                    }
                    _ => ts.err_here("expected number after '^' in type"),
                }
            } else {
                base_type
            }
        }
        Some(Token::LParen) => {
            // Tuple type: (T1, T2, ...)
            ts.next();
            let mut types = vec![];
            if !matches!(ts.peek(), Some(Token::RParen)) {
                loop {
                    types.push(parse_type(ts));
                    if !ts.eat(&Token::Comma) {
                        break;
                    }
                }
            }
            ts.expect(&Token::RParen, "tuple type ')'");
            Type::Tuple(types)
        }
        _ => ts.err_here("expected type annotation"),
    }
}

/// Parse specification (requires/ensures clauses)
fn parse_specification(ts: &mut Tokens) -> Specification {
    let mut requires = vec![];
    let mut ensures = vec![];

    // requires: expr
    if ts.eat(&Token::KwRequires) {
        ts.expect(&Token::Colon, "specification clause ':'");
        requires.push(parse_expr(ts));
    }

    // ensures: expr
    if ts.eat(&Token::KwEnsures) {
        ts.expect(&Token::Colon, "specification clause ':'");
        ensures.push(parse_expr(ts));
    }

    Specification { requires, ensures }
}

pub fn parse_module(ts: &mut Tokens) -> Module {
    let mut name: Option<String> = None;
    let mut imports = Vec::new();
    let mut uses = Vec::new();
    let mut exports = Vec::new();
    let mut reexports = Vec::new();
    let mut decls = Vec::new();

    // optional 'module Name'
    if ts.eat(&Token::KwModule) {
        let id = ts.expect_ident("module name");
        name = Some(id);
    }

    // any number of import/use/export/reexport
    loop {
        match ts.peek() {
            Some(Token::KwImport) => {
                ts.next();
                imports.push(parse_import(ts));
            }
            Some(Token::KwUse) => {
                ts.next();
                uses.extend(parse_use_list(ts));
            }
            Some(Token::KwExport) => {
                ts.next();
                exports.extend(parse_ident_list(ts));
            }
            Some(Token::KwReexport) => {
                ts.next();
                reexports.extend(parse_reexport_list(ts));
            }
            _ => break,
        }
        // optional semicolon
        ts.eat(&Token::Semicolon);
    }

    // declarations
    while let Some(tok) = ts.peek().cloned() {
        match tok {
            Token::At => {
                let d = parse_alg_def(ts);
                decls.push(TopLevelDecl::Definition(d));
            }
            Token::KwType => {
                let d = parse_type_def(ts);
                decls.push(TopLevelDecl::TypeDef(d));
            }
            Token::KwInclude => {
                ts.next();
                let path = ts.expect_string("include path");
                decls.push(TopLevelDecl::Include(path));
                ts.eat(&Token::Semicolon);
            }
            Token::EOF => break,
            _ => break,
        }
    }

    Module {
        name: name.unwrap_or_else(|| "Main".to_string()),
        imports,
        uses,
        exports,
        reexports,
        decls,
    }
}

fn parse_import(ts: &mut Tokens) -> Import {
    let path = parse_mod_path(ts);
    let alias = if ts.eat(&Token::KwAs) {
        Some(ts.expect_ident("import alias"))
    } else {
        None
    };
    Import { path, alias }
}

fn parse_use_list(ts: &mut Tokens) -> Vec<UseItem> {
    let mut items = Vec::new();
    loop {
        // Parse module path manually for use statements
        let path = parse_mod_path_for_use(ts);
        ts.expect(&Token::Dot, "'.' in use statement");

        if ts.eat(&Token::Star) {
            items.push(UseItem::Star { path });
        } else {
            let id = ts.expect_ident("name after path in use");
            items.push(UseItem::Named { path, ident: id });
        }

        if !ts.eat(&Token::Comma) {
            break;
        }
    }
    items
}

fn parse_mod_path_for_use(ts: &mut Tokens) -> ModPath {
    let mut segs = Vec::new();
    segs.push(ts.expect_ident("module path segment"));
    // Don't consume additional dots - let parse_use_list handle the final .name part
    ModPath { segments: segs }
}

fn parse_reexport_list(ts: &mut Tokens) -> Vec<ReexportItem> {
    let mut items = Vec::new();
    loop {
        let path = parse_mod_path(ts);
        ts.expect(&Token::Dot, "'.' before name to reexport");
        let id = ts.expect_ident("name to reexport");
        items.push(ReexportItem { path, ident: id });
        if !ts.eat(&Token::Comma) {
            break;
        }
    }
    items
}

fn parse_ident_list(ts: &mut Tokens) -> Vec<String> {
    let mut xs = Vec::new();
    loop {
        xs.push(ts.expect_ident("identifier"));
        if !ts.eat(&Token::Comma) {
            break;
        }
    }
    xs
}

fn parse_mod_path(ts: &mut Tokens) -> ModPath {
    let mut segs = Vec::new();
    segs.push(ts.expect_ident("module path segment"));
    while ts.eat(&Token::Dot) {
        segs.push(ts.expect_ident("module path segment"));
    }
    ModPath { segments: segs }
}

/* Expr := Let | Lambda | Match | Case | Pipe
   Let    := 'let' Ident '=' Expr 'in' Expr
   Lambda := '\' Ident {',' Ident} '->' Expr
           | 'fn' '(' [Ident {',' Ident}] ')' '=>' Expr
   Match  := 'match' Expr 'with' {'|' Pattern '->' Expr} 'end'
   Pipe   := Or { '>>' Or }
   Case   := '[' Arm {';' Arm} ']'
*/
pub fn parse_expr(ts: &mut Tokens) -> Expr {
    // Let binding has lowest precedence
    if let Some(Token::KwLet) = ts.peek() {
        return parse_let(ts);
    }
    // Lambda: \x -> expr or fn(x) => expr
    if let Some(Token::Backslash) = ts.peek() {
        return parse_lambda(ts);
    }
    if let Some(Token::KwFn) = ts.peek() {
        return parse_fn_lambda(ts);
    }
    // Match expression
    if let Some(Token::KwMatch) = ts.peek() {
        return parse_match(ts);
    }
    // Case has the next lowest precedence; check for it explicitly
    if let Some(Token::LBracket) = ts.peek() {
        return parse_case(ts);
    }
    // Set literal: { expr, expr, ... }
    if let Some(Token::LBrace) = ts.peek() {
        return parse_set(ts);
    }
    parse_pipe(ts)
}

/// Parse let binding: `let x = expr in body`
fn parse_let(ts: &mut Tokens) -> Expr {
    ts.expect(&Token::KwLet, "let keyword");
    let name = ts.expect_ident("variable name in let binding");
    ts.expect(&Token::Equal, "'=' in let binding");
    let value = parse_expr(ts);
    ts.expect(&Token::KwIn, "'in' keyword after let value");
    let body = parse_expr(ts);
    Expr::Let {
        name,
        value: Box::new(value),
        body: Box::new(body),
    }
}

/// Parse set literal: `{expr, expr, ...}` or empty `{}`
fn parse_set(ts: &mut Tokens) -> Expr {
    ts.expect(&Token::LBrace, "set '{'");
    let mut elements = Vec::new();

    if !matches!(ts.peek(), Some(Token::RBrace)) {
        elements.push(parse_expr(ts));
        while ts.eat(&Token::Comma) {
            // Allow trailing comma
            if matches!(ts.peek(), Some(Token::RBrace)) {
                break;
            }
            elements.push(parse_expr(ts));
        }
    }

    ts.expect(&Token::RBrace, "closing '}'");
    Expr::Set(elements)
}

/// Parse lambda: `\x -> expr` or `\x, y -> expr`
fn parse_lambda(ts: &mut Tokens) -> Expr {
    ts.expect(&Token::Backslash, "lambda '\\'");
    let params = parse_lambda_params(ts);
    ts.expect(&Token::Arrow, "'->' in lambda");
    let body = parse_expr(ts);
    Expr::Lambda {
        params,
        body: Box::new(body),
    }
}

/// Parse fn-style lambda: `fn(x) => expr` or `fn(x, y) => expr`
fn parse_fn_lambda(ts: &mut Tokens) -> Expr {
    ts.expect(&Token::KwFn, "'fn' keyword");
    ts.expect(&Token::LParen, "'(' in fn");
    let params = parse_lambda_params(ts);
    ts.expect(&Token::RParen, "')' in fn");
    ts.expect(&Token::FatArrow, "'=>' in fn");
    let body = parse_expr(ts);
    Expr::Lambda {
        params,
        body: Box::new(body),
    }
}

/// Parse comma-separated identifiers for lambda parameters
fn parse_lambda_params(ts: &mut Tokens) -> Vec<String> {
    let mut params = Vec::new();
    if let Some(Token::Ident(_)) = ts.peek() {
        params.push(ts.expect_ident("lambda parameter"));
        while ts.eat(&Token::Comma) {
            if matches!(ts.peek(), Some(Token::Arrow) | Some(Token::RParen)) {
                break; // trailing comma
            }
            params.push(ts.expect_ident("lambda parameter"));
        }
    }
    params
}

/// Parse match expression: `match expr with | Pat -> e1 | Pat2 -> e2 end`
fn parse_match(ts: &mut Tokens) -> Expr {
    ts.expect(&Token::KwMatch, "'match' keyword");
    let scrutinee = parse_expr(ts);
    ts.expect(&Token::KwWith, "'with' in match");

    let mut arms = Vec::new();
    while ts.eat(&Token::Pipe) {
        let pattern = parse_pattern(ts);
        ts.expect(&Token::Arrow, "'->' in match arm");
        let body = parse_expr(ts);
        arms.push(MatchArm { pattern, body });
    }

    ts.expect(&Token::KwEnd, "'end' to close match");

    Expr::Match {
        scrutinee: Box::new(scrutinee),
        arms,
    }
}

/// Parse a pattern for match expressions
fn parse_pattern(ts: &mut Tokens) -> Pattern {
    match ts.peek().cloned() {
        // Wildcard: _
        Some(Token::Ident(s)) if s == "_" => {
            ts.next();
            Pattern::Wildcard
        }
        // Number literal
        Some(Token::Number(s)) => {
            ts.next();
            let v: f64 = s.parse().unwrap_or(0.0);
            Pattern::Number(v)
        }
        // Boolean literal
        Some(Token::Bool(b)) => {
            ts.next();
            Pattern::Bool(b)
        }
        // String literal
        Some(Token::String(s)) => {
            ts.next();
            Pattern::String(s)
        }
        // Tuple pattern: (p1, p2, ...)
        Some(Token::LParen) => {
            ts.next();
            let mut pats = Vec::new();
            if !matches!(ts.peek(), Some(Token::RParen)) {
                pats.push(parse_pattern(ts));
                while ts.eat(&Token::Comma) {
                    if matches!(ts.peek(), Some(Token::RParen)) {
                        break;
                    }
                    pats.push(parse_pattern(ts));
                }
            }
            ts.expect(&Token::RParen, "')' in tuple pattern");
            Pattern::Tuple(pats)
        }
        // Variable binding or Constructor
        Some(Token::Ident(s)) => {
            ts.next();
            // Check if it's a constructor (starts with uppercase) with arguments
            if is_constructor_name(&s) {
                if ts.eat(&Token::LParen) {
                    let mut args = Vec::new();
                    if !matches!(ts.peek(), Some(Token::RParen)) {
                        args.push(parse_pattern(ts));
                        while ts.eat(&Token::Comma) {
                            if matches!(ts.peek(), Some(Token::RParen)) {
                                break;
                            }
                            args.push(parse_pattern(ts));
                        }
                    }
                    ts.expect(&Token::RParen, "')' in constructor pattern");
                    Pattern::Constructor { name: s, args }
                } else {
                    // Nullary constructor
                    Pattern::Constructor {
                        name: s,
                        args: vec![],
                    }
                }
            } else {
                // Variable binding (lowercase)
                Pattern::Var(s)
            }
        }
        other => ts.err_here(&format!("unexpected token in pattern: {:?}", other)),
    }
}

/// Check if a name looks like a constructor (starts with uppercase)
fn is_constructor_name(s: &str) -> bool {
    s.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
}

/// Parse type definition: `type Name<T> = Variant1 | Variant2(T)`
fn parse_type_def(ts: &mut Tokens) -> TypeDef {
    ts.expect(&Token::KwType, "'type' keyword");
    let name = ts.expect_ident("type name");

    // Optional type parameters: <T, U>
    let type_params = if ts.eat(&Token::Lt) {
        let mut params = vec![ts.expect_ident("type parameter")];
        while ts.eat(&Token::Comma) {
            params.push(ts.expect_ident("type parameter"));
        }
        ts.expect(&Token::Gt, "'>' to close type parameters");
        params
    } else {
        vec![]
    };

    ts.expect(&Token::Equal, "'=' in type definition");

    // Parse variants: Variant1 | Variant2(T) | ...
    let mut variants = vec![parse_variant(ts)];
    while ts.eat(&Token::Pipe) {
        variants.push(parse_variant(ts));
    }

    TypeDef {
        name,
        type_params,
        variants,
    }
}

/// Parse a single variant: `None` or `Some(T)`
fn parse_variant(ts: &mut Tokens) -> Variant {
    let name = ts.expect_ident("variant name");

    let fields = if ts.eat(&Token::LParen) {
        let mut fields = Vec::new();
        if !matches!(ts.peek(), Some(Token::RParen)) {
            fields.push(parse_type(ts));
            while ts.eat(&Token::Comma) {
                if matches!(ts.peek(), Some(Token::RParen)) {
                    break;
                }
                fields.push(parse_type(ts));
            }
        }
        ts.expect(&Token::RParen, "')' in variant");
        fields
    } else {
        vec![]
    };

    Variant { name, fields }
}

fn parse_case(ts: &mut Tokens) -> Expr {
    ts.expect(&Token::LBracket, "case '['");
    let mut arms: Vec<(Expr, Expr)> = Vec::new();
    let mut default: Option<Expr> = None;

    loop {
        // Check for default arm: _ (as identifier) or else keyword
        if ts.eat(&Token::KwElse) || is_underscore_ident(ts) {
            default = Some(parse_default_arm(ts));
        } else {
            parse_conditional_arm(ts, &mut arms);
        }

        if !ts.eat(&Token::Semicolon) {
            break;
        }
    }

    ts.expect(&Token::RBracket, "closing ']'");
    let def = default.expect("case block missing default 'else' or '_'");
    Expr::Case {
        arms,
        default: Box::new(def),
    }
}

/// Check if the next token is the `_` wildcard identifier and consume it
fn is_underscore_ident(ts: &mut Tokens) -> bool {
    if let Some(Token::Ident(s)) = ts.peek() {
        if s == "_" {
            ts.next();
            return true;
        }
    }
    false
}

fn parse_default_arm(ts: &mut Tokens) -> Expr {
    if ts.eat(&Token::QMark) || ts.eat(&Token::Arrow) {
        parse_expr(ts)
    } else {
        ts.err_here("expected '?' or '->' after 'else' or '_' in case arm")
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
        ts.err_here("expected '?' or '->' after condition in case arm");
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
    let head = parse_implies(ts);
    let mut steps: Vec<Expr> = Vec::new();
    while ts.eat(&Token::DblGt) {
        let step = parse_implies(ts);
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

/// Lowest-precedence boolean implication: A -> B, right-associative.
fn parse_implies(ts: &mut Tokens) -> Expr {
    let left = parse_or(ts);
    if ts.eat(&Token::Arrow) {
        let right = parse_implies(ts);
        Expr::Bin {
            op: BinOp::Implies,
            left: Box::new(left),
            right: Box::new(right),
        }
    } else {
        left
    }
}

/* precedence ladder: Or → And → Cmp → Add → Mul → Unary → Postfix → Primary
   Postfix here adds function calls after a primary:  name(args)  or  @Name(args)
*/

fn parse_or(ts: &mut Tokens) -> Expr {
    parse_binary_left_associative(ts, parse_and, &[(Token::DblPipe, BinOp::Or)])
}

fn parse_and(ts: &mut Tokens) -> Expr {
    parse_binary_left_associative(ts, parse_cmp, &[(Token::AmpAmp, BinOp::And)])
}

fn parse_cmp(ts: &mut Tokens) -> Expr {
    let mut node = parse_add(ts);
    let op = match ts.peek() {
        Some(Token::EqEq) | Some(Token::Equal) => Some(BinOp::Eq), // allow '=' as equality too
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
            (Token::KwMod, BinOp::Mod), // 'mod' keyword as alternative to %
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
        Some(Token::String(s)) => Expr::String(s),
        Some(Token::Ident(s)) => parse_qualified_name(ts, s, false),
        Some(Token::At) => parse_algorithm_call(ts),
        Some(Token::LParen) => parse_parenthesized(ts),
        other => ts.err_here(&format!("unexpected token in expression: {:?}", other)),
    }
}

/// Parse a constructor call: `Some(5)`, `Cons(1, tail)`
fn parse_constructor_call(ts: &mut Tokens, name: String) -> Expr {
    ts.expect(&Token::LParen, "'(' in constructor");
    let mut args = Vec::new();
    if !matches!(ts.peek(), Some(Token::RParen)) {
        args.push(parse_expr(ts));
        while ts.eat(&Token::Comma) {
            if matches!(ts.peek(), Some(Token::RParen)) {
                break;
            }
            args.push(parse_expr(ts));
        }
    }
    ts.expect(&Token::RParen, "')' in constructor");
    Expr::Constructor { name, args }
}

fn parse_number(ts: &mut Tokens, s: &str) -> Expr {
    let v: f64 = s
        .parse()
        .unwrap_or_else(|_| ts.err_here(&format!("bad number literal: {}", s)));
    Expr::Number(v)
}

/// Parse a potentially qualified name: `Name` or `Module.Name` or `Module.Sub.Name`
/// Returns either an Expr::Ident for simple names or Expr::Call with is_alg flag for qualified calls
fn parse_qualified_name(ts: &mut Tokens, first: String, is_alg: bool) -> Expr {
    let mut segments = vec![first];

    // Collect all dot-separated segments
    while ts.eat(&Token::Dot) {
        match ts.next() {
            Some(Token::Ident(s)) => segments.push(s),
            other => ts.err_here(&format!("expected identifier after '.', got {:?}", other)),
        }
    }

    // Join into a qualified name
    let name = segments.join(".");

    // If it's a qualified name (has dots), treat it as an algorithm reference
    if segments.len() > 1 {
        Expr::Call {
            is_alg: true, // qualified names are always algorithm references
            name,
            args: Vec::new(),
        }
    } else {
        // Simple unqualified name
        if is_alg {
            Expr::Call {
                is_alg: true,
                name,
                args: Vec::new(),
            }
        } else {
            Expr::Ident(name)
        }
    }
}

fn parse_algorithm_call(ts: &mut Tokens) -> Expr {
    let name = match ts.next() {
        Some(Token::Ident(s)) => s,
        other => ts.err_here(&format!("expected identifier after '@', got {:?}", other)),
    };
    // Handle qualified names after @: @Module.Function
    parse_qualified_name(ts, name, true)
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

fn attach_call_to_node(_ts: &mut Tokens, node: Expr, args: Vec<Expr>) -> Expr {
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
        // Lambda application
        Expr::Lambda { .. } => Expr::Apply {
            func: Box::new(node),
            args,
        },
        // Any other expression being applied - use Apply
        other => Expr::Apply {
            func: Box::new(other),
            args,
        },
    }
}
