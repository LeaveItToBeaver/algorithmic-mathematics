use crate::token::{TokSpan, Token, span};

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn peek2(bytes: &[u8], i: usize) -> Option<(char, char)> {
    if i + 1 >= bytes.len() {
        None
    } else {
        Some((bytes[i] as char, bytes[i + 1] as char))
    }
}

fn consume_block_content(bytes: &[u8], i0: usize) -> usize {
    // consume /* ... */ with nesting
    let mut i = i0 + 1; // after '/*'
    let len = bytes.len();
    let mut depth = 1i32;
    while i < len && depth > 0 {
        if i + 1 < len {
            let a = bytes[i] as char;
            let b = bytes[i + 1] as char;
            if a == '/' && b == '*' {
                depth += 1;
                i += 2;
                continue;
            }
            if a == '*' && b == '/' {
                depth -= 1;
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    i
}

fn keyword(text: &str) -> Option<Token> {
    match text {
        "module" => Some(Token::KwModule),
        "import" => Some(Token::KwImport),
        "use" => Some(Token::KwUse),
        "export" => Some(Token::KwExport),
        "reexport" => Some(Token::KwReexport),
        "include" => Some(Token::KwInclude),
        "as" => Some(Token::KwAs),
        "true" => Some(Token::Bool(true)),
        "false" => Some(Token::Bool(false)),
        _ => None,
    }
}

pub fn lex(input: &str) -> Vec<TokSpan> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i: usize = 0;
    let mut out: Vec<TokSpan> = Vec::new();

    while i < len {
        let b = bytes[i];

        // whitespace
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // two-char operators
        if let Some((a, c)) = peek2(bytes, i) {
            match (a, c) {
                ('-', '>') => {
                    out.push(span(Token::Arrow, i, i + 2));
                    i += 2;
                    continue;
                }
                ('|', '|') => {
                    out.push(span(Token::DblPipe, i, i + 2));
                    i += 2;
                    continue;
                }
                ('&', '&') => {
                    out.push(span(Token::AmpAmp, i, i + 2));
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
                ('>', '>') => {
                    out.push(span(Token::DblGt, i, i + 2));
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
                _ => {}
            }
        }

        // comments
        if let Some((a, c)) = peek2(bytes, i) {
            if a == '/' && c == '/' {
                // line comment
                i += 2;
                while i < len && (bytes[i] as char) != '\n' {
                    i += 1;
                }
                continue;
            }
            if a == '/' && c == '*' {
                i = consume_block_content(bytes, i);
                continue;
            }
        }

        // single-char punctuation
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
            '^' => {
                out.push(span(Token::Caret, i, i + 1));
                i += 1;
                continue;
            }
            '.' => {
                out.push(span(Token::Dot, i, i + 1));
                i += 1;
                continue;
            }
            '"' => {
                // string literal
                let start = i;
                i += 1;
                let mut s = String::new();
                while i < len {
                    let ch = bytes[i] as char;
                    if ch == '"' {
                        i += 1;
                        break;
                    }
                    if ch == '\\' {
                        i += 1;
                        if i >= len {
                            break;
                        }
                        let esc = bytes[i] as char;
                        s.push(match esc {
                            '\\' => '\\',
                            '"' => '"',
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            _ => esc,
                        });
                        i += 1;
                        continue;
                    }
                    s.push(ch);
                    i += 1;
                }
                out.push(span(Token::String(s), start, i));
                continue;
            }
            '%' => {
                out.push(span(Token::Percent, i, i + 1));
                i += 1;
                continue;
            }
            _ => {}
        }

        // identifier / keyword
        if is_ident_start(b as char) {
            let start = i;
            i += 1;
            while i < len && is_ident_continue(bytes[i] as char) {
                i += 1;
            }
            let text = &input[start..i];
            if let Some(kw) = keyword(text) {
                out.push(span(kw, start, i));
            } else {
                out.push(span(Token::Ident(text.to_string()), start, i));
            }
            continue;
        }

        // number
        if (b as char).is_ascii_digit() {
            let start = i;
            i += 1;
            while i < len && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
            if i < len && (bytes[i] as char) == '.' {
                let j = i + 1;
                // only treat as decimal if next is digit
                if j < len && (bytes[j] as char).is_ascii_digit() {
                    i += 1;
                    while i < len && (bytes[i] as char).is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let text = &input[start..i];
            out.push(span(Token::Number(text.to_string()), start, i));
            continue;
        }

        // unknown
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
