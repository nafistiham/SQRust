use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct AddColumnWithoutDefault;

impl Rule for AddColumnWithoutDefault {
    fn name(&self) -> &'static str {
        "Lint/AddColumnWithoutDefault"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip_set(source);
        let lower = source.to_lowercase();
        let bytes = lower.as_bytes();
        let len = bytes.len();
        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Skip positions inside strings or comments.
            if skip.contains(&i) {
                i += 1;
                continue;
            }

            // Look for "add" keyword (case-insensitive via lower).
            if !lower[i..].starts_with("add") {
                i += 1;
                continue;
            }

            let add_start = i;
            let add_end = i + 3;

            // Word-boundary check before "add".
            let before_ok = add_start == 0 || {
                let b = bytes[add_start - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            // Word-boundary check after "add".
            let after_ok = add_end >= len || {
                let b = bytes[add_end];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if !before_ok || !after_ok {
                i += 1;
                continue;
            }

            // Skip whitespace and check for "column" keyword.
            let mut j = add_end;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\r' || bytes[j] == b'\n') && !skip.contains(&j) {
                j += 1;
            }

            if j >= len || skip.contains(&j) || !lower[j..].starts_with("column") {
                i += 1;
                continue;
            }

            let column_end = j + 6;
            let column_after_ok = column_end >= len || {
                let b = bytes[column_end];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if !column_after_ok {
                i += 1;
                continue;
            }

            // Found "ADD COLUMN". Now find the end of the statement (semicolon or end-of-input).
            let stmt_start = add_start;
            let mut stmt_end = len;
            let mut k = column_end;
            while k < len {
                if !skip.contains(&k) && bytes[k] == b';' {
                    stmt_end = k;
                    break;
                }
                k += 1;
            }

            // Extract the statement slice (lowercased for keyword search).
            let stmt_lower = &lower[stmt_start..stmt_end];

            // Check if "default" appears in this statement with word boundaries.
            if !contains_word_boundary_keyword(stmt_lower, "default") {
                let (line, col) = offset_to_line_col(source, add_start);
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "ADD COLUMN without DEFAULT may cause a full table rewrite in some databases; consider adding a DEFAULT value".to_string(),
                    line,
                    col,
                });
            }

            // Advance past this statement.
            i = stmt_end + 1;
        }

        diags
    }
}

/// Returns `true` if `keyword` (already lowercased) appears with word boundaries in `text`.
fn contains_word_boundary_keyword(text: &str, keyword: &str) -> bool {
    let kw_len = keyword.len();
    let bytes = text.as_bytes();
    let text_len = bytes.len();
    let mut search_from = 0;

    while search_from < text_len {
        let Some(rel) = text[search_from..].find(keyword) else {
            break;
        };
        let abs = search_from + rel;

        let before_ok = abs == 0 || {
            let b = bytes[abs - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };
        let after = abs + kw_len;
        let after_ok = after >= text_len || {
            let b = bytes[after];
            !b.is_ascii_alphanumeric() && b != b'_'
        };

        if before_ok && after_ok {
            return true;
        }
        search_from = abs + 1;
    }

    false
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip set: byte offsets inside string literals, line comments, or block comments.
fn build_skip_set(source: &str) -> HashSet<usize> {
    let mut skip = HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Line comment: -- ... end-of-line
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            skip.insert(i);
            skip.insert(i + 1);
            i += 2;
            while i < len && bytes[i] != b'\n' {
                skip.insert(i);
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            skip.insert(i);
            skip.insert(i + 1);
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip.insert(i);
                    skip.insert(i + 1);
                    i += 2;
                    break;
                }
                skip.insert(i);
                i += 1;
            }
            continue;
        }

        // Single-quoted string: '...' with '' as escaped quote
        if bytes[i] == b'\'' {
            skip.insert(i);
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    skip.insert(i);
                    i += 1;
                    if i < len && bytes[i] == b'\'' {
                        skip.insert(i);
                        i += 1;
                        continue;
                    }
                    break;
                }
                skip.insert(i);
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..."
        if bytes[i] == b'"' {
            skip.insert(i);
            i += 1;
            while i < len && bytes[i] != b'"' {
                skip.insert(i);
                i += 1;
            }
            if i < len {
                skip.insert(i);
                i += 1;
            }
            continue;
        }

        // Backtick identifier: `...`
        if bytes[i] == b'`' {
            skip.insert(i);
            i += 1;
            while i < len && bytes[i] != b'`' {
                skip.insert(i);
                i += 1;
            }
            if i < len {
                skip.insert(i);
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}
