use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NStringLiteral;

impl Rule for NStringLiteral {
    fn name(&self) -> &'static str {
        "Convention/NStringLiteral"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

/// Build a skip set marking positions inside `--` line comments.
/// Single-quoted string positions are NOT marked here, because we need to find
/// the N that precedes the opening quote — the N itself is before the string starts.
fn build_comment_skip_set(source: &str) -> std::collections::HashSet<usize> {
    let mut skip = std::collections::HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\'' {
            // Skip past the string content so we don't accidentally detect `--`
            // inside strings as a comment.
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 2; // escaped quote
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
        } else if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            while i < len && bytes[i] != b'\n' {
                skip.insert(i);
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    skip
}

/// Build a skip set that marks positions INSIDE single-quoted strings.
/// This is used to skip the N character if it happens to appear inside a string
/// (though a bare N inside a string would not be followed by a quote in the outer
/// context, so this guard is a safety net).
fn build_string_skip_set(source: &str) -> std::collections::HashSet<usize> {
    let mut skip = std::collections::HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\'' {
            // mark the opening quote and everything inside
            skip.insert(i);
            i += 1;
            while i < len {
                skip.insert(i);
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }
    skip
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let comment_skip = build_comment_skip_set(source);
    let string_skip = build_string_skip_set(source);
    let mut diags = Vec::new();

    let mut i = 0;
    while i < len {
        // Skip positions inside line comments.
        if comment_skip.contains(&i) {
            i += 1;
            continue;
        }

        // Skip positions inside string literals.
        if string_skip.contains(&i) {
            i += 1;
            continue;
        }

        // Check for N' or n' pattern.
        if (bytes[i] == b'N' || bytes[i] == b'n') && i + 1 < len && bytes[i + 1] == b'\'' {
            // Ensure N is not part of a longer identifier (word boundary before N).
            let before_ok = i == 0 || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            if before_ok {
                let (line, col) = line_col(source, i);
                diags.push(Diagnostic {
                    rule: rule_name,
                    message: "N'...' national character literal is SQL Server-specific — use a regular string literal '...' for portable Unicode strings".to_string(),
                    line,
                    col,
                });
                // Advance past the N and the opening quote to avoid double-counting.
                i += 2;
                // Skip past the rest of the string so we don't re-scan its interior.
                while i < len {
                    if bytes[i] == b'\'' {
                        if i + 1 < len && bytes[i + 1] == b'\'' {
                            i += 2; // escaped quote
                        } else {
                            i += 1;
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                continue;
            }
        }

        i += 1;
    }

    diags
}
