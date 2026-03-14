use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct CommaAfterLastColumn;

impl Rule for CommaAfterLastColumn {
    fn name(&self) -> &'static str {
        "Layout/CommaAfterLastColumn"
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
    let mut line: usize = 1;
    let mut line_start: usize = 0;
    let mut i = 0;

    while i < len {
        if bytes[i] == b'\n' {
            line += 1;
            line_start = i + 1;
            i += 1;
            continue;
        }

        if skip[i] {
            i += 1;
            continue;
        }

        if bytes[i] == b',' {
            let comma_pos = i;
            let comma_line = line;
            let comma_col = comma_pos - line_start + 1;

            // Scan forward past whitespace/newlines/inline-comments to find the next token.
            let mut j = i + 1;
            while j < len {
                // Skip whitespace and newlines.
                if bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\r' || bytes[j] == b'\n' {
                    j += 1;
                    continue;
                }
                // Skip line comments (-- ...).
                if j + 1 < len && bytes[j] == b'-' && bytes[j + 1] == b'-' {
                    j += 2;
                    while j < len && bytes[j] != b'\n' {
                        j += 1;
                    }
                    continue;
                }
                // Skip block comments (/* ... */).
                if j + 1 < len && bytes[j] == b'/' && bytes[j + 1] == b'*' {
                    j += 2;
                    while j + 1 < len {
                        if bytes[j] == b'*' && bytes[j + 1] == b'/' {
                            j += 2;
                            break;
                        }
                        j += 1;
                    }
                    continue;
                }
                break;
            }

            // Check if the next non-whitespace token is FROM (word-bounded).
            if j + 4 <= len && bytes[j..j + 4].eq_ignore_ascii_case(b"FROM") {
                let after = j + 4;
                let after_ok = after >= len || !is_word_char(bytes[after]);
                if after_ok {
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: "Trailing comma before 'FROM' — remove the trailing comma after the last column".to_string(),
                        line: comma_line,
                        col: comma_col,
                    });
                }
            }

            i = comma_pos + 1;
            continue;
        }

        i += 1;
    }

    diags
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
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
