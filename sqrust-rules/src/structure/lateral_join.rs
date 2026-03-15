use sqrust_core::{Diagnostic, FileContext, Rule};

/// Flag `LATERAL` keyword in FROM clauses. LATERAL joins/subqueries are not
/// universally supported — they work in PostgreSQL and MySQL 8.0+ but are not
/// supported in SQL Server or older databases.
pub struct LateralJoin;

impl Rule for LateralJoin {
    fn name(&self) -> &'static str {
        "Structure/LateralJoin"
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
    let keyword = b"LATERAL";
    let kw_len = keyword.len();
    let mut diags = Vec::new();
    let mut i = 0;

    while i + kw_len <= len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Word boundary before.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Match LATERAL (case-insensitive).
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(keyword.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if !matches {
            i += 1;
            continue;
        }

        // Ensure none of those bytes are skipped.
        if (i..i + kw_len).any(|k| skip[k]) {
            i += 1;
            continue;
        }

        // Word boundary after.
        let after = i + kw_len;
        let after_ok = after >= len || !is_word_char(bytes[after]);

        if after_ok {
            let (line, col) = offset_to_line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: "LATERAL join is not supported in all databases (unsupported in SQL Server); verify dialect compatibility".to_string(),
                line,
                col,
            });
            i = after;
        } else {
            i += 1;
        }
    }

    diags
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Build a boolean skip-set: `skip[i] == true` means byte `i` is inside a
/// single-quoted string, double-quoted identifier, block comment, or line
/// comment.
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
