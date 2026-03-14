use sqrust_core::{Diagnostic, FileContext, Rule};

/// Flag aggregate functions other than COUNT that use `*` as their argument
/// (e.g., `SUM(*)`, `AVG(*)`, `MIN(*)`, `MAX(*)`). Only `COUNT(*)` is valid
/// SQL; using `*` with other aggregates is almost always a typo or logic error.
pub struct AggregateStar;

/// Aggregate function names that must NOT use `*`.
/// All lowercase for case-insensitive matching.
const FLAGGED_AGGREGATES: &[&str] = &[
    "sum", "avg", "min", "max", "stddev", "stddev_pop", "stddev_samp", "variance", "var_pop",
    "var_samp", "median",
];

impl Rule for AggregateStar {
    fn name(&self) -> &'static str {
        "Structure/AggregateStar"
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

        // Try to match each flagged aggregate starting at position i.
        let mut matched = false;
        for &func in FLAGGED_AGGREGATES {
            let flen = func.len();
            if i + flen + 2 > len {
                // Need at least funcname + "(*)"
                continue;
            }

            // Word-boundary before the function name.
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            if !before_ok {
                continue;
            }

            // Match function name (case-insensitive).
            let name_matches = bytes[i..i + flen]
                .iter()
                .zip(func.bytes())
                .all(|(a, b)| a.eq_ignore_ascii_case(&b));
            if !name_matches {
                continue;
            }

            // Immediately after the name must be '(' (word-boundary + paren).
            let paren_pos = i + flen;
            if paren_pos >= len || bytes[paren_pos] != b'(' {
                continue;
            }

            // After '(' must be '*'.
            let star_pos = paren_pos + 1;
            if star_pos >= len || bytes[star_pos] != b'*' {
                continue;
            }

            // After '*' must be ')'.
            let close_pos = star_pos + 1;
            if close_pos >= len || bytes[close_pos] != b')' {
                continue;
            }

            // None of these positions should be in a skip region.
            if skip[paren_pos] || skip[star_pos] || skip[close_pos] {
                continue;
            }

            // Build the display name (preserve case from source).
            let display_name: String = bytes[i..i + flen]
                .iter()
                .map(|b| b.to_ascii_uppercase() as char)
                .collect();

            let col = i - line_start + 1;
            diags.push(Diagnostic {
                rule: rule_name,
                message: format!(
                    "{display_name}(*) is not valid SQL — only COUNT(*) supports wildcard \
                     argument; use {display_name}(column_name) instead"
                ),
                line,
                col,
            });

            // Advance past the matched pattern so we don't double-count.
            i = close_pos + 1;
            matched = true;
            break;
        }

        if !matched {
            i += 1;
        }
    }

    diags
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
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
