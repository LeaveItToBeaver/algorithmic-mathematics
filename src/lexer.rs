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
        "else" => Some(Token::KwElse),
        "let" => Some(Token::KwLet),
        "in" => Some(Token::KwIn),
        "requires" => Some(Token::KwRequires),
        "ensures" => Some(Token::KwEnsures),
        "true" => Some(Token::Bool(true)),
        "false" => Some(Token::Bool(false)),
        "mod" => Some(Token::KwMod),  // modulo keyword (alternative to %)
        _ => None,
    }
}

/// Two-character operators, checked before single-char
const TWO_CHAR_OPS: &[((char, char), Token)] = &[
    ((':', '='), Token::Defines),
    (('-', '>'), Token::Arrow),
    (('|', '|'), Token::DblPipe),
    (('&', '&'), Token::AmpAmp),
    (('<', '='), Token::Le),
    (('>', '='), Token::Ge),
    (('>', '>'), Token::DblGt),
    (('=', '='), Token::EqEq),
    (('!', '='), Token::Neq),
];

/// Single-character tokens
const SINGLE_CHAR_OPS: &[(char, Token)] = &[
    ('@', Token::At),
    ('(', Token::LParen),
    (')', Token::RParen),
    ('[', Token::LBracket),
    (']', Token::RBracket),
    ('{', Token::LBrace),
    ('}', Token::RBrace),
    (',', Token::Comma),
    (';', Token::Semicolon),
    // ('_', Token::Underscore),  // Now treated as ident start
    ('=', Token::Equal),
    ('|', Token::Pipe),
    ('?', Token::QMark),
    ('!', Token::Bang),
    ('<', Token::Lt),
    ('>', Token::Gt),
    ('+', Token::Plus),
    ('-', Token::Minus),
    ('*', Token::Star),
    ('/', Token::Slash),
    ('^', Token::Caret),
    ('.', Token::Dot),
    (':', Token::Colon),
    ('%', Token::Percent),
];

pub fn lex(input: &str) -> Vec<TokSpan> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i: usize = 0;
    let mut out: Vec<TokSpan> = Vec::new();

    while i < len {
        let b = bytes[i];

        // Skip whitespace
        if b.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // Try two-char operators first (must check before single-char)
        if let Some((a, c)) = peek2(bytes, i) {
            // Comments take priority
            if a == '/' && c == '/' {
                i = skip_line_comment(bytes, i + 2, len);
                continue;
            }
            if a == '/' && c == '*' {
                i = consume_block_content(bytes, i);
                continue;
            }
            
            // Two-char operators
            if let Some(tok) = lookup_two_char(a, c) {
                out.push(span(tok, i, i + 2));
                i += 2;
                continue;
            }
        }

        // String literals
        if b as char == '"' {
            let (tok, end) = lex_string(bytes, i, len);
            out.push(span(tok, i, end));
            i = end;
            continue;
        }

        // Identifiers and keywords (includes _ as start)
        if is_ident_start(b as char) {
            let (tok, end) = lex_identifier(input, bytes, i, len);
            out.push(span(tok, i, end));
            i = end;
            continue;
        }

        // Numbers
        if (b as char).is_ascii_digit() {
            let (tok, end) = lex_number(input, bytes, i, len);
            out.push(span(tok, i, end));
            i = end;
            continue;
        }

        // Single-char operators
        if let Some(tok) = lookup_single_char(b as char) {
            out.push(span(tok, i, i + 1));
            i += 1;
            continue;
        }

        // Unknown character
        out.push(span(
            Token::Error(format!("unexpected character '{}'", b as char)),
            i,
            i + 1,
        ));
        i += 1;
    }

    out
}

fn lookup_two_char(a: char, c: char) -> Option<Token> {
    TWO_CHAR_OPS
        .iter()
        .find(|((x, y), _)| *x == a && *y == c)
        .map(|(_, tok)| tok.clone())
}

fn lookup_single_char(c: char) -> Option<Token> {
    SINGLE_CHAR_OPS
        .iter()
        .find(|(ch, _)| *ch == c)
        .map(|(_, tok)| tok.clone())
}

fn skip_line_comment(bytes: &[u8], start: usize, len: usize) -> usize {
    let mut i = start;
    while i < len && (bytes[i] as char) != '\n' {
        i += 1;
    }
    i
}

fn lex_string(bytes: &[u8], start: usize, len: usize) -> (Token, usize) {
    let mut i = start + 1; // skip opening quote
    let mut s = String::new();
    
    while i < len {
        let ch = bytes[i] as char;
        if ch == '"' {
            return (Token::String(s), i + 1);
        }
        if ch == '\\' && i + 1 < len {
            i += 1;
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
    
    (Token::String(s), i) // unterminated string
}

fn lex_identifier(input: &str, bytes: &[u8], start: usize, len: usize) -> (Token, usize) {
    let mut i = start + 1;
    while i < len && is_ident_continue(bytes[i] as char) {
        i += 1;
    }
    let text = &input[start..i];
    let tok = keyword(text).unwrap_or_else(|| Token::Ident(text.to_string()));
    (tok, i)
}

fn lex_number(input: &str, bytes: &[u8], start: usize, len: usize) -> (Token, usize) {
    let mut i = start + 1;
    
    // Integer part
    while i < len && (bytes[i] as char).is_ascii_digit() {
        i += 1;
    }
    
    // Decimal part (only if followed by digit, not method call like 3.14 vs obj.method)
    if i < len && (bytes[i] as char) == '.' {
        if i + 1 < len && (bytes[i + 1] as char).is_ascii_digit() {
            i += 1; // consume '.'
            while i < len && (bytes[i] as char).is_ascii_digit() {
                i += 1;
            }
        }
    }
    
    let text = &input[start..i];
    (Token::Number(text.to_string()), i)
}
