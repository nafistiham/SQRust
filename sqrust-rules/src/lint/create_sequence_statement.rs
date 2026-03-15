use sqrust_core::{Diagnostic, FileContext, Rule};
use std::collections::HashSet;

pub struct CreateSequenceStatement;

impl Rule for CreateSequenceStatement {
    fn name(&self) -> &'static str {
        "Lint/CreateSequenceStatement"
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

            // Look for "create" keyword (case-insensitive via lower).
            if !lower[i..].starts_with("create") {
                i += 1;
                continue;
            }

            let create_start = i;
            let create_end = i + 6;

            // Word-boundary check before "create".
            let before_ok = create_start == 0 || {
                let b = bytes[create_start - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            // Word-boundary check after "create".
            let after_ok = create_end >= len || {
                let b = bytes[create_end];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if !before_ok || !after_ok {
                i += 1;
                continue;
            }

            // Skip whitespace to find next keyword.
            let mut j = create_end;
            while j < len
                && (bytes[j] == b' '
                    || bytes[j] == b'\t'
                    || bytes[j] == b'\r'
                    || bytes[j] == b'\n')
                && !skip.contains(&j)
            {
                j += 1;
            }

            // Skip over optional "OR REPLACE" (word boundaries checked loosely — just skip if present).
            // Not needed for sequences, but skip "IF NOT EXISTS" tokens or just check for "sequence".
            // Directly check for the "sequence" keyword (allowing "IF NOT EXISTS" between CREATE and SEQUENCE
            // by scanning forward for "sequence" with word boundaries before the next semicolon or EOF).
            let stmt_end = find_stmt_end(&lower, &skip, j);
            let stmt_slice = &lower[j..stmt_end];

            if contains_word_boundary_keyword(stmt_slice, "sequence") {
                let (line, col) = offset_to_line_col(source, create_start);
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "CREATE SEQUENCE is not universally supported; MySQL uses AUTO_INCREMENT, SQLite uses AUTOINCREMENT — check dialect compatibility".to_string(),
                    line,
                    col,
                });

                // Advance past this statement.
                i = stmt_end + 1;
                continue;
            }

            i += 1;
        }

        diags
    }
}

/// Finds the byte offset of the next `;` (outside skip) starting from `from`,
/// or the length of the source if no `;` is found.
fn find_stmt_end(lower: &str, skip: &HashSet<usize>, from: usize) -> usize {
    let bytes = lower.as_bytes();
    let len = bytes.len();
    let mut k = from;
    while k < len {
        if !skip.contains(&k) && bytes[k] == b';' {
            return k;
        }
        k += 1;
    }
    len
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
