use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct NoTableHint;

const HINTS: &[&str] = &[
    "NOLOCK", "READPAST", "UPDLOCK", "HOLDLOCK", "TABLOCK", "ROWLOCK", "PAGLOCK", "XLOCK",
    "NOEXPAND",
];

impl Rule for NoTableHint {
    fn name(&self) -> &'static str {
        "Convention/NoTableHint"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip_set(source);
        let mut diags = Vec::new();

        for hint in HINTS {
            // Build the pattern: ( + hint + )
            let pattern = format!("({})", hint.to_lowercase());
            let lower = source.to_lowercase();
            let pat_len = pattern.len();
            let bytes = lower.as_bytes();
            let len = bytes.len();
            let mut i = 0;
            while i + pat_len <= len {
                if !skip.contains(&i) && lower[i..].starts_with(&pattern) {
                    // Check all bytes in the match are not in the skip set
                    let all_code = (i..i + pat_len).all(|k| !skip.contains(&k));
                    if all_code {
                        // Check word boundary after closing paren (or at end)
                        let after = i + pat_len;
                        let after_ok = after >= len || {
                            let b = bytes[after];
                            !b.is_ascii_alphanumeric() && b != b'_'
                        };
                        if after_ok {
                            // Extract the actual-case hint from the original source
                            // hint portion starts at i+1, length = hint.len()
                            let hint_start = i + 1;
                            let hint_end = hint_start + hint.len();
                            let actual_hint = &source[hint_start..hint_end];
                            let (line, col) = offset_to_line_col(source, i);
                            diags.push(Diagnostic {
                                rule: self.name(),
                                message: format!(
                                    "Table hint WITH ({actual_hint}) is SQL Server-specific; \
                                     use transaction isolation levels instead \
                                     (e.g. SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED)"
                                ),
                                line,
                                col,
                            });
                        }
                    }
                }
                i += 1;
            }
        }

        diags.sort_by_key(|d| (d.line, d.col));
        diags
    }
}

fn build_skip_set(source: &str) -> HashSet<usize> {
    let mut skip = HashSet::new();
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

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
