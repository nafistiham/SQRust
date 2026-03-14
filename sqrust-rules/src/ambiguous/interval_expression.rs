use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct IntervalExpression;

const INTERVAL_KEYWORD: &[u8] = b"INTERVAL";
const INTERVAL_KEYWORD_LEN: usize = INTERVAL_KEYWORD.len();

impl Rule for IntervalExpression {
    fn name(&self) -> &'static str {
        "Ambiguous/IntervalExpression"
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

    while i + INTERVAL_KEYWORD_LEN <= len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Word boundary before INTERVAL
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if before_ok && bytes[i..i + INTERVAL_KEYWORD_LEN].eq_ignore_ascii_case(INTERVAL_KEYWORD) {
            let after = i + INTERVAL_KEYWORD_LEN;
            // Word boundary after: the character immediately after must not be a word char
            // This ensures `INTERVAL_DAYS` (identifier containing INTERVAL) is not flagged.
            let after_ok = after >= len || !is_word_char(bytes[after]);
            if after_ok {
                let (line, col) = line_col(source, i);
                diags.push(Diagnostic {
                    rule: rule_name,
                    message: "INTERVAL expression syntax varies across databases — \
                               SQL Server uses DATEADD(), MySQL/BigQuery use INTERVAL N UNIT, \
                               PostgreSQL uses INTERVAL 'N unit'; consider using a date \
                               arithmetic function abstraction"
                        .to_string(),
                    line,
                    col,
                });
                i += INTERVAL_KEYWORD_LEN;
                continue;
            }
        }

        i += 1;
    }

    diags
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
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
