use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoCharType;

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip table: `true` at every byte inside strings, comments, or
/// quoted identifiers. These positions must be ignored by the scanner.
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

        // Single-quoted string: '...' with '' escape (SQL standard)
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

/// Returns `true` if `ch` is a SQL word character (`[a-zA-Z0-9_]`).
#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Scans `source` for bare `CHAR` keyword occurrences (case-insensitive) that
/// are outside strings/comments and are not part of a longer word (e.g.
/// VARCHAR, NCHAR, CHARVAR).
///
/// Returns the byte offset of each `CHAR` keyword found.
fn find_char_offsets(source: &str, skip: &[bool]) -> Vec<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let pattern = b"CHAR";
    let pat_len = pattern.len(); // 4
    let mut results = Vec::new();
    let mut i = 0;

    while i + pat_len <= len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Case-insensitive match of "CHAR"
        let matches = bytes[i..i + pat_len]
            .iter()
            .zip(pattern.iter())
            .all(|(&a, &b)| a.eq_ignore_ascii_case(&b));

        if matches {
            // Verify all matched bytes are in code (not skip) region
            let all_code = (i..i + pat_len).all(|k| !skip[k]);

            if all_code {
                // Word boundary before: preceding char must not be a word char
                let boundary_before = i == 0 || !is_word_char(bytes[i - 1]);
                // Word boundary after: following char must not be a word char
                let end = i + pat_len;
                let boundary_after = end >= len || !is_word_char(bytes[end]);

                if boundary_before && boundary_after {
                    results.push(i);
                    i += pat_len;
                    continue;
                }
            }
        }

        i += 1;
    }

    results
}

impl Rule for NoCharType {
    fn name(&self) -> &'static str {
        "Convention/NoCharType"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        let bytes = source.as_bytes();
        let skip = build_skip(bytes);
        let offsets = find_char_offsets(source, &skip);

        offsets
            .into_iter()
            .map(|offset| {
                let (line, col) = line_col(source, offset);
                Diagnostic {
                    rule: self.name(),
                    message: "CHAR type used; prefer VARCHAR for variable-length strings"
                        .to_string(),
                    line,
                    col,
                }
            })
            .collect()
    }
}
