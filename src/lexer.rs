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

fn consume_block_content(bytes: &[u8], mut i: usize) -> usize {
        let len = bytes.len();
        let mut depth = 1usize;
        // `i` points at the first '*' in "/*", move past it
        i += 1;

        while i < len {
                let c = bytes[i] as char;
                if c == '/' && i + 1 < len && (bytes[i + 1] as char) == '*' {
                        depth += 1;
                        i += 2;
                        continue;
                }
                if c == '*' && i + 1 < len && (bytes[i + 1] as char) == '/' {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                                break;
                        }
                        continue;
                }
                i += 1;
        }
        i
}
fn lex_string_literal(bytes: &[u8], _input: &str, start: usize, out: &mut Vec<TokSpan>) -> usize {
        let mut i = start + 1; // skip opening quote
        let len = bytes.len();
        let mut s = String::new();

        while i < len {
                let ch = bytes[i] as char;
                i += 1;

                if ch == '"' {
                        out.push(span(Token::String(s), start, i));
                        return i;
                }

                if ch == '\\' && i < len {
                        i = process_escape_sequence(bytes, i, &mut s);
                } else {
                        s.push(ch);
                }
        }

        // Unterminated string
        out.push(span(
                Token::Error("unterminated string literal".into()),
                start,
                i,
        ));
        i
}

fn process_escape_sequence(bytes: &[u8], i: usize, s: &mut String) -> usize {
        let esc = bytes[i] as char;
        match esc {
                '\\' => s.push('\\'),
                '"' => s.push('"'),
                'n' => s.push('\n'),
                't' => s.push('\t'),
                'r' => s.push('\r'),
                _ => s.push(esc),
        }
        i + 1
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

                // comments
                if let Some((a, c)) = peek2(bytes, i) {
                        if a == '/' && c == '/' {
                                // // line comment: skip until newline
                                i += 2;
                                while i < len && (bytes[i] as char) != '\n' {
                                        i += 1;
                                }
                                continue;
                        }
                        if a == '/' && c == '*' {
                                i = consume_block_content(bytes, i + 1);
                                continue;
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
                        i = lex_string_literal(bytes, input, i, &mut out);
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
