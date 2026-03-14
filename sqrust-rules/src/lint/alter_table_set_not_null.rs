use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct AlterTableSetNotNull;

impl Rule for AlterTableSetNotNull {
    fn name(&self) -> &'static str {
        "Lint/AlterTableSetNotNull"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let skip = build_skip(bytes);
        let upper = source.to_uppercase();

        let mut diags = Vec::new();

        // Scan for "SET NOT NULL" and "DROP NOT NULL" outside strings/comments.
        // These patterns are only valid inside ALTER TABLE statements, so any
        // occurrence in active SQL code is flagged.
        for (pattern, message) in &[
            (
                "SET NOT NULL",
                "SET NOT NULL is PostgreSQL-specific ALTER TABLE syntax \
                 — use ALTER COLUMN with NOT NULL constraint definition \
                 for portable column constraint changes",
            ),
            (
                "DROP NOT NULL",
                "DROP NOT NULL is PostgreSQL-specific ALTER TABLE syntax \
                 — use dialect-specific constraint modification syntax \
                 or recreate the column",
            ),
        ] {
            let pat_len = pattern.len();
            let upper_bytes = upper.as_bytes();
            let text_len = upper_bytes.len();
            let mut search_from = 0usize;

            while search_from < text_len {
                let Some(rel) = upper[search_from..].find(pattern) else {
                    break;
                };
                let abs = search_from + rel;

                // Check if this position is inside a string or comment — skip if so.
                if skip[abs] {
                    search_from = abs + 1;
                    continue;
                }

                // Word-boundary check before the pattern.
                let before_ok = abs == 0 || {
                    let b = upper_bytes[abs - 1];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };

                // Word-boundary check after the pattern (after the last char).
                let after = abs + pat_len;
                let after_ok = after >= text_len || {
                    let b = upper_bytes[after];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };

                if before_ok && after_ok {
                    let (line, col) = offset_to_line_col(source, abs);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: message.to_string(),
                        line,
                        col,
                    });
                }

                search_from = abs + 1;
            }
        }

        diags
    }
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
