use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct PivotUnpivot;

impl Rule for PivotUnpivot {
    fn name(&self) -> &'static str {
        "Convention/PivotUnpivot"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

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

    // Search for PIVOT keyword (7 bytes) and UNPIVOT keyword (8 bytes).
    let pivot_kw = b"PIVOT";
    let pivot_len = pivot_kw.len();
    let unpivot_kw = b"UNPIVOT";
    let unpivot_len = unpivot_kw.len();

    let mut i = 0;
    while i < len {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        // Try to match UNPIVOT first (longer keyword, must check before PIVOT to avoid
        // matching the PIVOT portion of UNPIVOT).
        if i + unpivot_len <= len {
            let before_ok = i == 0 || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            if before_ok
                && bytes[i..i + unpivot_len].eq_ignore_ascii_case(unpivot_kw)
            {
                let after = i + unpivot_len;
                let after_ok = after >= len || {
                    let b = bytes[after];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
                if after_ok {
                    let (line, col) = line_col(source, i);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: "UNPIVOT is a non-standard SQL extension (SQL Server/Oracle/Snowflake) — use UNION ALL or dbt_utils.unpivot() for portable unpivoting".to_string(),
                        line,
                        col,
                    });
                    i += unpivot_len;
                    continue;
                }
            }
        }

        // Try to match PIVOT (but only if not part of UNPIVOT — already consumed above).
        if i + pivot_len <= len {
            let before_ok = i == 0 || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            if before_ok
                && bytes[i..i + pivot_len].eq_ignore_ascii_case(pivot_kw)
            {
                let after = i + pivot_len;
                let after_ok = after >= len || {
                    let b = bytes[after];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
                if after_ok {
                    let (line, col) = line_col(source, i);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: "PIVOT is a non-standard SQL extension (SQL Server/Oracle/Snowflake) — use CASE WHEN or dbt_utils.pivot() for portable pivoting".to_string(),
                        line,
                        col,
                    });
                    i += pivot_len;
                    continue;
                }
            }
        }

        i += 1;
    }

    diags
}
