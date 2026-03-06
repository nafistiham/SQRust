use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct ConcatOperator;

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip table: `true` at every byte inside strings, comments, or
/// quoted identifiers.
fn build_skip(bytes: &[u8]) -> Vec<bool> {
    let len = bytes.len();
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Line comment: -- ... newline
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

        // Single-quoted string: '...' with '' escape
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    skip[i] = true;
                    i += 1;
                    if i < len && bytes[i] == b'\'' {
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    break;
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

/// Scans `source` for `||` operators outside strings/comments.
/// Returns the byte offset of the first `|` for each occurrence.
fn find_concat_offsets(source: &str, skip: &[bool]) -> Vec<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut results = Vec::new();
    let mut i = 0;

    while i + 1 < len {
        if skip[i] {
            i += 1;
            continue;
        }

        if bytes[i] == b'|' && bytes[i + 1] == b'|' && !skip[i + 1] {
            results.push(i);
            // Skip both bytes to avoid re-matching the second `|`
            i += 2;
            continue;
        }

        i += 1;
    }

    results
}

impl Rule for ConcatOperator {
    fn name(&self) -> &'static str {
        "Convention/ConcatOperator"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        let bytes = source.as_bytes();
        let skip = build_skip(bytes);
        let offsets = find_concat_offsets(source, &skip);

        offsets
            .into_iter()
            .map(|offset| {
                let (line, col) = line_col(source, offset);
                Diagnostic {
                    rule: self.name(),
                    message: "Use CONCAT() instead of || for cross-database portability"
                        .to_string(),
                    line,
                    col,
                }
            })
            .collect()
    }
}
