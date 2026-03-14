use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct DropColumnIfExists;

impl Rule for DropColumnIfExists {
    fn name(&self) -> &'static str {
        "Lint/DropColumnIfExists"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip_set(source);
        let lower = source.to_lowercase();
        let bytes = lower.as_bytes();
        let len = bytes.len();

        let mut diags = Vec::new();

        // Search for "drop column" occurrences not inside strings/comments.
        let pattern = "drop column";
        let pat_len = pattern.len();
        let mut i = 0;

        while i + pat_len <= len {
            if !skip.contains(&i) && lower[i..].starts_with(pattern) {
                // Verify word boundary before "drop"
                let before_ok = i == 0 || {
                    let b = bytes[i - 1];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
                // Verify word boundary after "column"
                let after_pos = i + pat_len;
                let after_ok = after_pos >= len || {
                    let b = bytes[after_pos];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };

                if before_ok && after_ok {
                    // Check whether "IF EXISTS" appears between "DROP" and "COLUMN"
                    // or after "COLUMN" (before end of statement).
                    // Strategy: scan backwards from "drop" to see if "if exists" precedes
                    // "column", or forward from "column" to see if "if exists" follows it.

                    // Check for "DROP IF EXISTS COLUMN" pattern (some dialects allow this,
                    // though uncommon) and "DROP COLUMN IF EXISTS" pattern (standard).
                    //
                    // The simplest correct approach: look for "if exists" anywhere in the
                    // region from "drop" (or earlier on the same statement segment) to the
                    // end of the statement. We search the window from i-20 to
                    // the next semicolon (or end of source).
                    let stmt_end = find_stmt_end(&lower, i);
                    let window = &lower[i..stmt_end];

                    // "drop column if exists" — IF EXISTS after COLUMN
                    // "drop if exists column" — IF EXISTS between DROP and COLUMN
                    let has_if_exists = window.contains("if exists");

                    if !has_if_exists {
                        let (line, col) = offset_to_line_col(source, i);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: "DROP COLUMN without IF EXISTS may fail if the column does \
                                      not exist; consider using IF EXISTS"
                                .to_string(),
                            line,
                            col,
                        });
                    }
                }
                i += pat_len;
            } else {
                i += 1;
            }
        }

        diags
    }
}

/// Returns the byte index just past the end of the current statement (i.e. the
/// position of the next `;` or the end of the source).
fn find_stmt_end(lower: &str, from: usize) -> usize {
    lower[from..]
        .find(';')
        .map(|rel| from + rel)
        .unwrap_or(lower.len())
}

fn build_skip_set(source: &str) -> HashSet<usize> {
    let mut skip = HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\'' {
            // Single-quoted string — mark every byte inside as skip
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        // Escaped quote inside string
                        skip.insert(i);
                        i += 2;
                    } else {
                        // End of string
                        i += 1;
                        break;
                    }
                } else {
                    skip.insert(i);
                    i += 1;
                }
            }
        } else if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            // Line comment — mark until end of line
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
