#[derive(Debug, Clone, PartialEq)]
pub enum Token {
        // punctuation / structure
        At,
        LParen,
        RParen,
        LBracket,
        RBracket,
        Comma,
        Semicolon,
        Underscore,
        Equal,
        Arrow,
        Pipe,
        QMark,
        DblPipe,
        DblAmp,
        DblGt,
        Plus,
        Minus,
        Star,
        Slash,
        Percent,
        EqEq,
        Neq,
        Le,
        Ge,
        Lt,
        Gt,
        Bang,
        Ident(String),
        Number(String),
        Bool(bool),
        String(String),

        // unknown
        Error(String),

        Caret,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TokSpan {
        pub tok: Token,
        pub start: usize,
        pub end: usize,
}

pub fn span(tok: Token, start: usize, end: usize) -> TokSpan {
        TokSpan { tok, start, end }
}

pub fn caret_message(src: &str, byte: usize, msg: &str) -> String {
        let mut line = 1usize;
        let mut col = 1usize;
        let mut last_nl = 0usize;
        for (i, ch) in src.char_indices() {
                if i >= byte {
                        break;
                }
                if ch == '\n' {
                        line += 1;
                        last_nl = i + 1;
                        col = 1;
                } else {
                        col += 1
                }
        }
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
