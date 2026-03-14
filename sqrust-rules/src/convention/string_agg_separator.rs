use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct StringAggSeparator;

impl Rule for StringAggSeparator {
    fn name(&self) -> &'static str {
        "Convention/StringAggSeparator"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

/// Builds a set of byte offsets that should be skipped (inside string literals or
/// line comments).
fn build_skip_set(source: &str) -> std::collections::HashSet<usize> {
    let mut skip = std::collections::HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\'' {
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        skip.insert(i);
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    skip.insert(i);
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

    let skip = build_skip_set(source);
    let mut diags = Vec::new();

    let group_concat_kw = b"GROUP_CONCAT";
    let group_concat_len = group_concat_kw.len();
    let listagg_kw = b"LISTAGG";
    let listagg_len = listagg_kw.len();

    let mut i = 0;
    while i < len {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        // Try to match GROUP_CONCAT
        if i + group_concat_len <= len {
            let before_ok = i == 0 || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            if before_ok && bytes[i..i + group_concat_len].eq_ignore_ascii_case(group_concat_kw) {
                let after = i + group_concat_len;
                let after_ok = after >= len || {
                    let b = bytes[after];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
                if after_ok {
                    let (line, col) = line_col(source, i);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: "GROUP_CONCAT() is MySQL-specific — use STRING_AGG(col, separator) for portable string aggregation (PostgreSQL, SQL Server, BigQuery)".to_string(),
                        line,
                        col,
                    });
                    i += group_concat_len;
                    continue;
                }
            }
        }

        // Try to match LISTAGG
        if i + listagg_len <= len {
            let before_ok = i == 0 || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            if before_ok && bytes[i..i + listagg_len].eq_ignore_ascii_case(listagg_kw) {
                let after = i + listagg_len;
                let after_ok = after >= len || {
                    let b = bytes[after];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
                if after_ok {
                    let (line, col) = line_col(source, i);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: "LISTAGG() is Oracle/Snowflake-specific — use STRING_AGG(col, separator) for portable string aggregation".to_string(),
                        line,
                        col,
                    });
                    i += listagg_len;
                    continue;
                }
            }
        }

        i += 1;
    }

    diags
}
