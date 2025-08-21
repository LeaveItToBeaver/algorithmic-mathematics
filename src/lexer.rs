// src/lexer.rs
use crate::token::{TokSpan, Token, span};

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub fn lex(input: &str) -> Vec<TokSpan> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i: usize = 0;
    let mut out: Vec<TokSpan> = Vec::new();

    while i < len {
        let b = bytes[i];

        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // two-char operators first
        if i + 1 < len {
            let a = bytes[i] as char;
            let c = bytes[i + 1] as char;
            match (a, c) {
                ('-', '>') => {
                    out.push(span(Token::Arrow, i, i + 2));
                    i += 2;
                    continue;
                }
                ('>', '>') => {
                    out.push(span(Token::DblGt, i, i + 2));
                    i += 2;
                    continue;
                }
                ('|', '|') => {
                    out.push(span(Token::DblPipe, i, i + 2));
                    i += 2;
                    continue;
                }
                ('&', '&') => {
                    out.push(span(Token::DblAmp, i, i + 2));
                    i += 2;
                    continue;
                }
                ('=', '=') => {
                    out.push(span(Token::EqEq, i, i + 2));
                    i += 2;
                    continue;
                }
                ('!', '=') => {
                    out.push(span(Token::Neq, i, i + 2));
                    i += 2;
                    continue;
                }
                ('<', '=') => {
                    out.push(span(Token::Le, i, i + 2));
                    i += 2;
                    continue;
                }
                ('>', '=') => {
                    out.push(span(Token::Ge, i, i + 2));
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }

        // single-char
        match b as char {
            '@' => {
                out.push(span(Token::At, i, i + 1));
                i += 1;
                continue;
            }
            '(' => {
                out.push(span(Token::LParen, i, i + 1));
                i += 1;
                continue;
            }
            ')' => {
                out.push(span(Token::RParen, i, i + 1));
                i += 1;
                continue;
            }
            '[' => {
                out.push(span(Token::LBracket, i, i + 1));
                i += 1;
                continue;
            }
            ']' => {
                out.push(span(Token::RBracket, i, i + 1));
                i += 1;
                continue;
            }
            ',' => {
                out.push(span(Token::Comma, i, i + 1));
                i += 1;
                continue;
            }
            ';' => {
                out.push(span(Token::Semicolon, i, i + 1));
                i += 1;
                continue;
            }
            '_' => {
                out.push(span(Token::Underscore, i, i + 1));
                i += 1;
                continue;
            }
            '=' => {
                out.push(span(Token::Equal, i, i + 1));
                i += 1;
                continue;
            }
            '|' => {
                out.push(span(Token::Pipe, i, i + 1));
                i += 1;
                continue;
            }
            '?' => {
                out.push(span(Token::QMark, i, i + 1));
                i += 1;
                continue;
            }
            '!' => {
                out.push(span(Token::Bang, i, i + 1));
                i += 1;
                continue;
            }
            '+' => {
                out.push(span(Token::Plus, i, i + 1));
                i += 1;
                continue;
            }
            '-' => {
                out.push(span(Token::Minus, i, i + 1));
                i += 1;
                continue;
            }
            '*' => {
                out.push(span(Token::Star, i, i + 1));
                i += 1;
                continue;
            }
            '/' => {
                out.push(span(Token::Slash, i, i + 1));
                i += 1;
                continue;
            }
            '%' => {
                out.push(span(Token::Percent, i, i + 1));
                i += 1;
                continue;
            }
            '<' => {
                out.push(span(Token::Lt, i, i + 1));
                i += 1;
                continue;
            }
            '>' => {
                out.push(span(Token::Gt, i, i + 1));
                i += 1;
                continue;
            }
            '^' => {
                out.push(span(Token::Caret, i, i + 1));
                i += 1;
                continue;
            }
            _ => {}
        }

        let ch = bytes[i] as char;

        if (bytes[i] as char) == '"' {
            let start = i;
            i += 1; // skip opening quote
            let mut s = String::new();
            while i < len {
                let ch = bytes[i] as char;
                i += 1;
                if ch == '"' {
                    // closing quote
                    out.push(span(Token::String(s), start, i));
                    break;
                }
                if ch == '\\' && i < len {
                    let esc = bytes[i] as char;
                    i += 1;
                    match esc {
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        'r' => s.push('\r'),
                        _ => s.push(esc),
                    }
                } else {
                    s.push(ch);
                }
            }
            // if we fell out without hitting a closing '"', emit an Error token
            if let Some(TokSpan {
                tok: Token::String(_),
                ..
            }) = out.last()
            {
                // ok
            } else {
                out.push(span(
                    Token::Error("unterminated string literal".into()),
                    start,
                    i,
                ));
            }
            continue;
        }

        // identifier / keyword
        if is_ident_start(ch) {
            let start = i;
            i += 1;
            while i < len && is_ident_continue(bytes[i] as char) {
                i += 1;
            }
            let text = &input[start..i];
            let tok = match text {
                "true" => Token::Bool(true),
                "false" => Token::Bool(false),
                _ => Token::Ident(text.to_string()),
            };
            out.push(span(tok, start, i));
            continue;
        }

        // number
        if ch.is_ascii_digit() {
            let start = i;
            i += 1;
            while i < len && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
            if i < len && (bytes[i] as char) == '.' {
                i += 1;
                while i < len && (bytes[i] as char).is_ascii_digit() {
                    i += 1;
                }
            }
            let text = &input[start..i];
            out.push(span(Token::Number(text.to_string()), start, i));
            continue;
        }

        // unknown → error token
        let start = i;
        let bad = bytes[i] as char;
        i += 1;
        out.push(span(
            Token::Error(format!("unexpected character '{}'", bad)),
            start,
            i,
        ));
    }

    out
}
