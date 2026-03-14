use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoSpaceBeforeOpenParen;

/// SQL keywords that can legitimately appear before `(` with whitespace.
/// Mirrors the list in `FunctionCallSpacing` — these are syntax constructs,
/// not function calls.
const KEYWORD_BEFORE_PAREN: &[&str] = &[
    "IN", "NOT", "EXISTS", "AS", "ON", "BETWEEN", "HAVING", "WHERE", "WHEN", "THEN", "ELSE",
    "FROM", "JOIN", "UNION", "INTERSECT", "EXCEPT", "SELECT", "BY", "PARTITION", "OVER",
    "WITHIN", "AND", "OR", "CASE", "IF",
];

impl Rule for NoSpaceBeforeOpenParen {
    fn name(&self) -> &'static str {
        "Layout/NoSpaceBeforeOpenParen"
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

        // Look for: word_char followed by one-or-more tabs followed by `(`.
        // The current byte must be a tab, not in a skip region, and
        // preceded by a word character.
        if b == b'\t' && !skip[i] && i > 0 {
            let prev = bytes[i - 1];
            if is_word_char(prev) {
                // Find the start of the word token before this tab.
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

                // Skip past all consecutive tabs (must all be outside skip region).
                let tab_start = i;
                let mut j = i;
                while j < len && bytes[j] == b'\t' && !skip[j] {
                    j += 1;
                }

                // Next non-tab char must be `(` and outside skip region.
                if j < len && bytes[j] == b'(' && !skip[j] {
                    let word = std::str::from_utf8(&bytes[word_start..word_end])
                        .unwrap_or("")
                        .to_uppercase();

                    if !is_keyword_before_paren(&word) {
                        let col = tab_start - line_start + 1;
                        diags.push(Diagnostic {
                            rule: rule_name,
                            message: format!(
                                "Function call '{}' has a tab character before '(' — write '{}(...)' without whitespace",
                                word, word
                            ),
                            line,
                            col,
                        });
                        // Advance past the tabs.
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
