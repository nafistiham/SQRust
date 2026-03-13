use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct FunctionCallSpacing;

/// SQL keywords that can legitimately appear directly before `(` with a space.
/// These are syntax constructs, not function calls.
const KEYWORD_BEFORE_PAREN: &[&str] = &[
    "IN", "NOT", "EXISTS", "AS", "ON", "BETWEEN", "HAVING", "WHERE", "WHEN", "THEN", "ELSE",
    "FROM", "JOIN", "UNION", "INTERSECT", "EXCEPT", "SELECT", "BY", "PARTITION", "OVER",
    "WITHIN", "AND", "OR", "CASE", "IF",
];

impl Rule for FunctionCallSpacing {
    fn name(&self) -> &'static str {
        "Layout/FunctionCallSpacing"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(bytes, len);
    let mut diags = Vec::new();
    let mut i = 0;
    let mut line: usize = 1;
    let mut line_start: usize = 0;

    while i < len {
        let b = bytes[i];

        if b == b'\n' {
            line += 1;
            line_start = i + 1;
            i += 1;
            continue;
        }

        // Look for: word_char followed by one-or-more spaces followed by '('
        // The current byte must be a space, and we need to check the char before it
        // was a word char, and chars after (skipping spaces) is '('.
        if b == b' ' && !skip[i] && i > 0 {
            // Check that the byte immediately before this space is a word character
            // (letter, digit, or underscore) and NOT the start of a line.
            let prev = bytes[i - 1];
            if is_word_char(prev) {
                // Find the start of the word token before this space.
                let word_end = i; // exclusive
                let mut word_start = i;
                while word_start > 0 && is_word_char(bytes[word_start - 1]) {
                    word_start -= 1;
                }

                // Ensure the word itself is not in a skip region.
                if skip[word_start] {
                    i += 1;
                    continue;
                }

                // Skip past all consecutive spaces (must all be outside skip region).
                let space_start = i;
                let mut j = i;
                while j < len && bytes[j] == b' ' && !skip[j] {
                    j += 1;
                }

                // Next non-space char must be '(' and outside skip region.
                if j < len && bytes[j] == b'(' && !skip[j] {
                    // Extract the word token.
                    let word =
                        std::str::from_utf8(&bytes[word_start..word_end]).unwrap_or("").to_uppercase();

                    if !is_keyword_before_paren(&word) {
                        let col = space_start - line_start + 1;
                        diags.push(Diagnostic {
                            rule: rule_name,
                            message: format!(
                                "Function call '{}' has a space before '(' — use '{}(...)' without space",
                                word, word
                            ),
                            line,
                            col,
                        });
                        // Advance past the spaces so we don't re-trigger on the same run.
                        i = j;
                        continue;
                    }
                }
            }
        }

        i += 1;
    }

    diags
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_keyword_before_paren(word: &str) -> bool {
    KEYWORD_BEFORE_PAREN.contains(&word)
}

/// Build a boolean skip-set: `skip[i] == true` means byte `i` is inside a
/// single-quoted string, double-quoted identifier, block comment, or line comment.
fn build_skip_set(bytes: &[u8], len: usize) -> Vec<bool> {
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Single-quoted string: '...' with '' escape.
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..." with "" escape.
        if bytes[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'"' {
                    if i + 1 < len && bytes[i + 1] == b'"' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len {
                skip[i] = true;
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[i + 1] = true;
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Line comment: -- to end of line.
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len && bytes[i] != b'\n' {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}
