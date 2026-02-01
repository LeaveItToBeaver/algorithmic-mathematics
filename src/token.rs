#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // punctuation / structure
    At,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,      // '{'
    RBrace,      // '}'
    Comma,
    Semicolon,
    Underscore,
    Equal,
    Arrow,       // '->'
    FatArrow,    // '=>' for lambdas
    Defines,     // ':='
    Pipe,
    QMark,
    DblPipe,
    AmpAmp,
    Bang,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Dot,
    DblGt,   // '>>'
    EqEq,    // '=='
    Neq,     // '!='
    Percent, // '%'
    Colon,   // ':'
    Backslash, // '\' for lambdas

    // literals / names
    Ident(String),
    Number(String),
    String(String),
    Bool(bool),

    // keywords
    KwModule,
    KwImport,
    KwUse,
    KwExport,
    KwReexport,
    KwInclude,
    KwAs,
    KwElse,
    KwLet,
    KwIn,
    KwRequires,
    KwEnsures,
    KwMod,
    // NEW: ADT and pattern matching keywords
    KwType,      // 'type' for ADT definitions
    KwMatch,     // 'match' for pattern matching
    KwWith,      // 'with' in match expressions
    KwEnd,       // 'end' to close match
    KwFn,        // 'fn' for lambda alternative

    // misc
    EOF,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct TokSpan {
    pub tok: Token,
    pub span: Span,
}

pub fn span(tok: Token, start: usize, end: usize) -> TokSpan {
    TokSpan {
        tok,
        span: Span { start, end },
    }
}

pub fn caret_message(src: &str, byte: usize, msg: &str) -> String {
    let mut line = 1usize;
    let mut last_nl = 0usize;
    for (i, ch) in src.char_indices() {
        if i >= byte {
            break;
        }
        if ch == '\n' {
            line += 1;
            last_nl = i + 1;
        }
    }
    let col = byte - last_nl + 1;
    let line_end = src[last_nl..]
        .find('\n')
        .map(|x| last_nl + x)
        .unwrap_or(src.len());
    let line_text = &src[last_nl..line_end];

    let mut caret = String::new();
    for _ in 1..col {
        caret.push(' ');
    }
    caret.push('^');

    format!("error: {msg} \n --> input:{line}:{col}\n{line:>3} | {line_text}\n | {caret} here")
}
