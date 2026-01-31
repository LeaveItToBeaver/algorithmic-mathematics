/// Normalize Unicode mathematical notation to ASCII
/// 
/// This allows mathematicians to paste formulas from papers/LaTeX
/// and have them work directly. The canonical representation is ASCII
/// (for diffs, typing), but we accept Unicode as input.
pub fn normalize_unicode_to_ascii(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for ch in src.chars() {
        match ch {
            // Whitespace normalization
            '\u{00A0}' => out.push(' '),  // non-breaking space

            // Logical operators
            '\u{2227}' => out.push_str("&&"),  // ∧ AND
            '\u{2228}' => out.push_str("||"),  // ∨ OR
            '\u{00AC}' => out.push('!'),        // ¬ NOT

            // Comparison operators
            '\u{2260}' => out.push_str("!="),  // ≠
            '\u{2264}' => out.push_str("<="),  // ≤
            '\u{2265}' => out.push_str(">="),  // ≥
            '\u{2261}' => out.push_str("=="),  // ≡ (identical)
            
            // Arrows
            '\u{2192}' | '\u{21D2}' => out.push_str("->"),  // → ⇒
            '\u{2190}' => out.push_str("<-"),              // ←
            
            // Definition
            '\u{225D}' | '\u{2254}' => out.push_str(":="), // ≝ ≔
            
            // Arithmetic
            '\u{2212}' => out.push('-'),       // − (minus sign)
            '\u{00D7}' | '\u{2217}' | '\u{22C5}' => out.push('*'),  // × ∗ ⋅
            '\u{00F7}' => out.push('/'),       // ÷

            // Special values
            '\u{221E}' => out.push_str("inf"), // ∞
            '\u{03C0}' => out.push_str("pi"),  // π
            '\u{03C4}' => out.push_str("tau"), // τ
            '\u{212F}' => out.push('e'),       // ℯ (Euler's e)

            // Type symbols (often used in math)
            '\u{211D}' => out.push('R'),       // ℝ
            '\u{2115}' => out.push('N'),       // ℕ
            '\u{2124}' => out.push('Z'),       // ℤ
            '\u{211A}' => out.push('Q'),       // ℚ
            '\u{2102}' => out.push('C'),       // ℂ
            '\u{1D539}' => out.push_str("Bool"), // 𝔹

            // Quantifiers (for specs)
            '\u{2200}' => out.push_str("forall"),  // ∀
            '\u{2203}' => out.push_str("exists"),  // ∃
            '\u{2208}' => out.push_str(" in "),    // ∈
            '\u{2286}' => out.push_str("subset"),  // ⊆
            
            // Square root
            '\u{221A}' => out.push_str("sqrt"),    // √

            // Superscript digits → ^ notation
            '\u{2070}' => out.push_str("^0"),
            '\u{00B9}' => out.push_str("^1"),
            '\u{00B2}' => out.push_str("^2"),
            '\u{00B3}' => out.push_str("^3"),
            '\u{2074}' => out.push_str("^4"),
            '\u{2075}' => out.push_str("^5"),
            '\u{2076}' => out.push_str("^6"),
            '\u{2077}' => out.push_str("^7"),
            '\u{2078}' => out.push_str("^8"),
            '\u{2079}' => out.push_str("^9"),
            '\u{207F}' => out.push_str("^n"),  // ⁿ

            // Default: keep as-is
            _ => out.push(ch),
        }
    }
    out
}
