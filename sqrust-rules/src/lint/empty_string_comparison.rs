use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct EmptyStringComparison;

impl Rule for EmptyStringComparison {
    fn name(&self) -> &'static str {
        "Lint/EmptyStringComparison"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = build_skip(bytes);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Only examine bytes outside strings/comments.
            if skip[i] {
                i += 1;
                continue;
            }

            // Try to match one of: `!=`, `<>`, `=` (in that order so we
            // don't accidentally consume just the `!` or `<` alone).
            let (op_len, op_start) = if i + 1 < len
                && bytes[i] == b'!'
                && bytes[i + 1] == b'='
                && !skip[i + 1]
            {
                (2, i)
            } else if i + 1 < len
                && bytes[i] == b'<'
                && bytes[i + 1] == b'>'
                && !skip[i + 1]
            {
                (2, i)
            } else if bytes[i] == b'=' {
                (1, i)
            } else {
                i += 1;
                continue;
            };

            let after_op = op_start + op_len;

            // Skip whitespace after operator (outside skip regions).
            let mut j = after_op;
            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                j += 1;
            }

            // Check for empty string: two consecutive single quotes.
            // The skip table marks both quotes as `true` (they are the
            // delimiters of an empty string literal), so we test the raw
            // bytes directly — the operator position guards us against
            // being inside a comment or string already.
            if j + 1 < len && bytes[j] == b'\'' && bytes[j + 1] == b'\'' {
                // Make sure this isn't the start of a longer string (e.g. `'''`)
                // — three quotes would mean an escaped-quote string, not empty.
                // Actually `'''` in SQL is the string `'` (one quote char).
                // We still flag because the VALUE being compared to is a
                // single-quote character, not truly empty; however the plan
                // specifically calls out `'it''s'` as NOT flagged (escaped
                // quote inside a string). The skip table handles that: inside
                // `'it''s'`, position of the inner `''` is already in the skip
                // region because the outer string started at the first `'`.
                // Here we are OUTSIDE any skip region (the operator was outside
                // skip), so `bytes[j]` is the start of a new literal.
                // Two consecutive quotes with nothing else = empty string.
                let (line, col) = line_col(source, op_start);
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "Comparison with empty string; consider checking for NULL as well"
                        .to_string(),
                    line,
                    col,
                });

                // Advance past the operator and empty string so we don't
                // re-scan them.
                i = j + 2;
                continue;
            }

            i += op_len;
        }

        diags
    }
}

/// Returns `true` if `ch` is a word character (`[a-zA-Z0-9_]`).
#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip table: `true` for every byte offset that is inside a
/// string literal, line comment, block comment, or quoted identifier.
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

    // Suppress unused-import lint — `is_word_char` is defined for potential
    // future use (e.g. word-boundary guards around operators).
    let _ = is_word_char;

    skip
}
