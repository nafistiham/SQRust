use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct AlterTableAddNotNullWithoutDefault;

impl Rule for AlterTableAddNotNullWithoutDefault {
    fn name(&self) -> &'static str {
        "Lint/AlterTableAddNotNullWithoutDefault"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = build_skip(bytes);

        let mut diags = Vec::new();
        let upper = source.to_uppercase();
        let upper_bytes = upper.as_bytes();

        let mut i = 0;
        while i < len {
            // Skip positions inside strings or comments.
            if skip[i] {
                i += 1;
                continue;
            }

            // Look for "ALTER" keyword (word-boundary, case-insensitive via upper).
            if !upper[i..].starts_with("ALTER") {
                i += 1;
                continue;
            }

            let alter_start = i;
            let alter_end = i + 5; // "ALTER".len()

            // Word-boundary check before ALTER.
            let before_ok = alter_start == 0 || {
                let b = upper_bytes[alter_start - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            // Word-boundary check after ALTER.
            let after_ok = alter_end >= len || {
                let b = upper_bytes[alter_end];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if !before_ok || !after_ok {
                i += 1;
                continue;
            }

            // Skip whitespace and check for TABLE keyword.
            let mut j = alter_end;
            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                j += 1;
            }

            if j >= len || skip[j] || !upper[j..].starts_with("TABLE") {
                i += 1;
                continue;
            }

            let table_end = j + 5; // "TABLE".len()
            let table_after_ok = table_end >= len || {
                let b = upper_bytes[table_end];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if !table_after_ok {
                i += 1;
                continue;
            }

            // We have ALTER TABLE. Now find the end of this statement (semicolon or end-of-input),
            // staying outside skip regions for the semicolon check but extracting the full
            // statement text for analysis.
            let stmt_start = alter_start;
            let mut stmt_end = len;
            let mut k = table_end;
            while k < len {
                if !skip[k] && bytes[k] == b';' {
                    stmt_end = k;
                    break;
                }
                k += 1;
            }

            // Extract the statement slice (upper-cased for keyword search).
            let stmt_upper = &upper[stmt_start..stmt_end];

            // Check if NOT NULL appears in this statement (word-boundary).
            let not_null_offset = find_word_boundary_keyword(stmt_upper, "NOT NULL");

            if let Some(rel_offset) = not_null_offset {
                // Check if DEFAULT also appears in this statement (word-boundary).
                // If DEFAULT is present, do not flag — the column has a fallback value.
                if !contains_word_boundary_keyword(stmt_upper, "DEFAULT") {
                    // Report position of NOT NULL within the original source.
                    let abs_offset = stmt_start + rel_offset;
                    let (line, col) = offset_to_line_col(source, abs_offset);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message:
                            "Adding a NOT NULL column without DEFAULT will fail on non-empty tables"
                                .to_string(),
                        line,
                        col,
                    });
                }
            }

            // Advance past this statement.
            i = stmt_end + 1;
        }

        diags
    }
}

/// Searches for `keyword` (already uppercased, may contain a space like "NOT NULL")
/// with word-boundary checks on both ends. Returns the byte offset of the first match
/// relative to `text`, or `None`.
fn find_word_boundary_keyword(text: &str, keyword: &str) -> Option<usize> {
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
            return Some(abs);
        }
        search_from = abs + 1;
    }

    None
}

/// Returns `true` if `keyword` (already uppercased) appears with word boundaries in `text`.
fn contains_word_boundary_keyword(text: &str, keyword: &str) -> bool {
    find_word_boundary_keyword(text, keyword).is_some()
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip table: `true` for every byte offset inside a string literal,
/// line comment, or block comment.
fn build_skip(bytes: &[u8]) -> Vec<bool> {
    let len = bytes.len();
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Line comment: -- ... end-of-line
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

        // Block comment: /* ... */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[i] = true;
                    skip[i + 1] = true;
                    i += 2;
                    break;
                }
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Single-quoted string: '...' with '' as escaped quote
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    skip[i] = true;
                    i += 1;
                    // '' inside a string is an escaped quote — continue in string
                    if i < len && bytes[i] == b'\'' {
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    break; // end of string
                }
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..."
        if bytes[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len && bytes[i] != b'"' {
                skip[i] = true;
                i += 1;
            }
            if i < len {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Backtick identifier: `...`
        if bytes[i] == b'`' {
            skip[i] = true;
            i += 1;
            while i < len && bytes[i] != b'`' {
                skip[i] = true;
                i += 1;
            }
            if i < len {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}
