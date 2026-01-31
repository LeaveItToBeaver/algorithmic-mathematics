/// Render ASCII source to Unicode for display/PDF export
/// This is the inverse of normalize.rs - we keep ASCII as canonical,
/// but render it beautifully for human consumption.

pub fn render_unicode(src: &str) -> String {
    let mut out = String::with_capacity(src.len() * 2);
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Two-character sequences first (greedy matching)
        if i + 1 < len {
            match (chars[i], chars[i + 1]) {
                // Definition and arrows
                (':', '=') => {
                    out.push('≝');
                    i += 2;
                    continue;
                }
                ('-', '>') => {
                    out.push('→');
                    i += 2;
                    continue;
                }

                // Comparisons
                ('!', '=') => {
                    out.push('≠');
                    i += 2;
                    continue;
                }
                ('<', '=') => {
                    out.push('≤');
                    i += 2;
                    continue;
                }
                ('>', '=') => {
                    out.push('≥');
                    i += 2;
                    continue;
                }

                // Logical operators
                ('&', '&') => {
                    out.push('∧');
                    i += 2;
                    continue;
                }
                ('|', '|') => {
                    out.push('∨');
                    i += 2;
                    continue;
                }

                // Type identifiers with superscripts (R^3 -> ℝ³)
                (ty, '^') if is_type_char(ty) => {
                    out.push(render_type_char(ty));
                    i += 2;

                    // Collect superscript digits/variable
                    if i < len {
                        if chars[i].is_ascii_digit() {
                            out.push(superscript_digit(chars[i]));
                            i += 1;
                        } else if chars[i].is_ascii_alphabetic() {
                            // R^n -> ℝⁿ (for variables)
                            out.push(superscript_letter(chars[i]));
                            i += 1;
                        }
                    }
                    continue;
                }

                _ => {}
            }
        }

        // Three+ character sequences (keywords)
        if i + 6 <= len {
            let slice: String = chars[i..i + 6].iter().collect();
            if slice == "forall" {
                out.push('∀');
                i += 6;
                continue;
            }
            if slice == "exists" {
                out.push('∃');
                i += 6;
                continue;
            }
            if slice == "subset" {
                out.push('⊆');
                i += 6;
                continue;
            }
        }

        if i + 4 <= len {
            let slice: String = chars[i..i + 4].iter().collect();
            if slice == "sqrt" {
                out.push('√');
                i += 4;
                continue;
            }
        }

        if i + 3 <= len {
            let slice: String = chars[i..i + 3].iter().collect();
            if slice == "inf" {
                out.push('∞');
                i += 3;
                continue;
            }
            if slice == "tau" {
                out.push('τ');
                i += 3;
                continue;
            }
        }

        if i + 2 <= len {
            let slice: String = chars[i..i + 2].iter().collect();
            if slice == "pi" {
                out.push('π');
                i += 2;
                continue;
            }
            if slice == "in" && (i == 0 || !chars[i - 1].is_alphanumeric()) {
                // Only render 'in' as ∈ if it's standalone (not part of identifier)
                if i + 2 >= len || !chars[i + 2].is_alphanumeric() {
                    out.push('∈');
                    i += 2;
                    continue;
                }
            }
        }

        // Single type characters in type context
        if is_type_char(chars[i]) && is_type_context(&chars, i) {
            out.push(render_type_char(chars[i]));
            i += 1;
            continue;
        }

        // Contextual multiplication (× instead of *)
        if chars[i] == '*' && is_multiplication_context(&chars, i) {
            out.push('×');
            i += 1;
            continue;
        }

        // Contextual dot product (⋅ instead of .)
        if chars[i] == '.' && is_dot_product_context(&chars, i) {
            out.push('⋅');
            i += 1;
            continue;
        }

        // Default: keep character as-is
        out.push(chars[i]);
        i += 1;
    }

    out
}

/// Check if character is a type identifier (R, N, Z, Q, C)
fn is_type_char(c: char) -> bool {
    matches!(c, 'R' | 'N' | 'Z' | 'Q' | 'C')
}

/// Render type character to Unicode
fn render_type_char(c: char) -> char {
    match c {
        'R' => 'ℝ',
        'N' => 'ℕ',
        'Z' => 'ℤ',
        'Q' => 'ℚ',
        'C' => 'ℂ',
        _ => c,
    }
}

/// Convert ASCII digit to superscript
fn superscript_digit(c: char) -> char {
    match c {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        _ => c,
    }
}

/// Convert ASCII letter to superscript (for variables like R^n)
fn superscript_letter(c: char) -> char {
    match c {
        'n' => 'ⁿ',
        'i' => 'ⁱ',
        // Add more as needed
        _ => c,
    }
}

/// Check if we're in a type annotation context
/// e.g., after ':' or '->', or in generic brackets
fn is_type_context(chars: &[char], i: usize) -> bool {
    // Look backwards for type indicators
    let mut j = i;
    while j > 0 {
        j -= 1;
        match chars[j] {
            ' ' | '\t' => continue, // Skip whitespace
            ':' => return true,     // After colon in param: Type
            ',' => return true,     // In list of types
            '<' => return true,     // In generic bracket Set<Type>
            '>' => return true,     // After closing bracket
            '(' => return false,    // Start of params, not type context
            _ => {
                if chars[j].is_alphanumeric() || chars[j] == '_' {
                    continue; // Part of identifier
                }
                return false;
            }
        }
    }

    // Look forward
    if i + 1 < chars.len() {
        matches!(
            chars[i + 1],
            ',' | '>' | ')' | '-' | '^' | ' ' | '\t' | '\n'
        )
    } else {
        false
    }
}

/// Check if * is used for multiplication (not pointer/glob)
fn is_multiplication_context(chars: &[char], i: usize) -> bool {
    // * is multiplication if surrounded by expressions
    let has_left = i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == ')');
    let has_right = i + 1 < chars.len() && (chars[i + 1].is_alphanumeric() || chars[i + 1] == '(');
    has_left && has_right
}

/// Check if . is used for dot product (not member access or decimal)
fn is_dot_product_context(chars: &[char], i: usize) -> bool {
    // . is dot product if surrounded by spaces or identifiers (not digits)
    let has_left = i > 0 && !chars[i - 1].is_ascii_digit();
    let has_right = i + 1 < chars.len() && !chars[i + 1].is_ascii_digit();
    has_left && has_right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_rendering() {
        assert_eq!(render_unicode("a := b"), "a ≝ b");
        assert_eq!(render_unicode("a -> b"), "a → b");
        assert_eq!(render_unicode("a != b"), "a ≠ b");
        assert_eq!(render_unicode("a <= b"), "a ≤ b");
        assert_eq!(render_unicode("a >= b"), "a ≥ b");
        assert_eq!(render_unicode("a && b"), "a ∧ b");
        assert_eq!(render_unicode("a || b"), "a ∨ b");
    }

    #[test]
    fn test_types() {
        assert_eq!(render_unicode("f(x: R) -> R"), "f(x: ℝ) → ℝ");
        assert_eq!(render_unicode("a: N, b: Z"), "a: ℕ, b: ℤ");
        assert_eq!(render_unicode("Set<R>"), "Set<ℝ>");
    }

    #[test]
    fn test_superscripts() {
        assert_eq!(render_unicode("R^3"), "ℝ³");
        assert_eq!(render_unicode("R^n"), "ℝⁿ");
        // Note: x^2 doesn't get superscripted because x is not a type char
        // This is intentional - only type identifiers (R, N, Z, Q, C) get special treatment
        assert_eq!(render_unicode("x^2"), "x^2");
    }

    #[test]
    fn test_constants() {
        assert_eq!(render_unicode("inf"), "∞");
        assert_eq!(render_unicode("pi"), "π");
        assert_eq!(render_unicode("tau"), "τ");
    }

    #[test]
    fn test_quantifiers() {
        assert_eq!(render_unicode("forall x"), "∀ x");
        assert_eq!(render_unicode("exists y"), "∃ y");
        assert_eq!(render_unicode("x in S"), "x ∈ S");
    }

    #[test]
    fn test_full_algorithm() {
        let input = "@SafeDiv(a: R, b: R) -> R := [\n  b != 0 -> a/b;\n  else -> inf\n]";
        let expected = "@SafeDiv(a: ℝ, b: ℝ) → ℝ ≝ [\n  b ≠ 0 → a/b;\n  else → ∞\n]";
        assert_eq!(render_unicode(input), expected);
    }
}
