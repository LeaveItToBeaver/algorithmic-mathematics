pub fn normalize_unicode_to_ascii(src: &str) -> String {
        let mut out = String::with_capacity(src.len());
        for ch in src.chars() {
                match ch {
                        '\u{00A0}' => out.push(' '),

                        '\u{2227}' => {
                                out.push('&');
                                out.push('&');
                        }
                        '\u{2228}' => {
                                out.push('|');
                                out.push('|');
                        }

                        '\u{00AC}' => out.push('!'),

                        '\u{2260}' => {
                                out.push('!');
                                out.push('=');
                        }
                        '\u{2264}' => {
                                out.push('<');
                                out.push('=');
                        }
                        '\u{2265}' => {
                                out.push('>');
                                out.push('=');
                        }
                        '\u{2192}' | '\u{21D2}' => {
                                out.push('-');
                                out.push('>');
                        }
                        '\u{2212}' => out.push('-'),
                        '\u{00D7}' | '\u{2217}' => out.push('*'),
                        '\u{00F7}' => out.push('/'),

                        '\u{221E}' => {
                                out.push_str("inf");
                        }
                        '\u{2261}' => {
                                out.push('=');
                                out.push('=');
                        }
                        _ => out.push(ch),
                }
        }
        out
}
