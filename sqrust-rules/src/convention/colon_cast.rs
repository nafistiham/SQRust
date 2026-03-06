use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct ColonCast;

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Scans `source` for PostgreSQL-style `::type` casts outside string literals.
///
/// Returns a list of byte offsets where `::` was found, where:
/// - The `::` is not inside a single-quoted string literal (with `''` escaping).
/// - The byte immediately following `::` is ASCII alphabetic or `_` (a type name start).
/// - The `::` is not part of a triple-colon `:::` sequence.
fn find_colon_casts(source: &str) -> Vec<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut results = Vec::new();
    let mut i = 0;

    while i < len {
        // Line comment: -- ... end-of-line; skip to newline.
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Single-quoted string: '...' with '' as escaped quote.
        if bytes[i] == b'\'' {
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    i += 1;
                    // '' is an escaped quote inside the string — continue scanning.
                    if i < len && bytes[i] == b'\'' {
                        i += 1;
                        continue;
                    }
                    // Closing quote found.
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..."
        if bytes[i] == b'"' {
            i += 1;
            while i < len && bytes[i] != b'"' {
                i += 1;
            }
            if i < len {
                i += 1; // consume the closing '"'
            }
            continue;
        }

        // Backtick identifier: `...`
        if bytes[i] == b'`' {
            i += 1;
            while i < len && bytes[i] != b'`' {
                i += 1;
            }
            if i < len {
                i += 1; // consume the closing '`'
            }
            continue;
        }

        // Detect `::` at position i.
        if i + 1 < len && bytes[i] == b':' && bytes[i + 1] == b':' {
            // Reject `:::` — a third colon makes this a different token.
            if i + 2 < len && bytes[i + 2] == b':' {
                i += 1;
                continue;
            }

            // The byte after `::` must be alphabetic or `_` to indicate a type name.
            let after = i + 2;
            if after < len && (bytes[after].is_ascii_alphabetic() || bytes[after] == b'_') {
                results.push(i);
                i += 2; // advance past the `::`
                continue;
            }
        }

        i += 1;
    }

    results
}

impl Rule for ColonCast {
    fn name(&self) -> &'static str {
        "Convention/ColonCast"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let offsets = find_colon_casts(source);

        offsets
            .into_iter()
            .map(|offset| {
                let (line, col) = line_col(source, offset);
                Diagnostic {
                    rule: self.name(),
                    message: "PostgreSQL :: cast; use CAST(expr AS type) for portability"
                        .to_string(),
                    line,
                    col,
                }
            })
            .collect()
    }
}
