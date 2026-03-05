use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct BooleanComparison;

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

/// Checks whether `bytes[offset..]` starts with `pattern` case-insensitively,
/// followed by a word boundary (end of input or non-alphanumeric/non-underscore).
fn bool_keyword_at(bytes: &[u8], offset: usize, pattern: &[u8]) -> bool {
    let end = offset + pattern.len();
    if end > bytes.len() {
        return false;
    }
    let matches = bytes[offset..end]
        .iter()
        .zip(pattern.iter())
        .all(|(&a, &b)| a.eq_ignore_ascii_case(&b));
    if !matches {
        return false;
    }
    // Word boundary after the keyword
    if end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        return false;
    }
    true
}

/// Scans `source` for `= TRUE`, `= FALSE`, `<> TRUE`, `<> FALSE`,
/// `!= TRUE`, `!= FALSE` patterns outside strings/comments.
/// Returns the byte offset of the operator (`=`, `<>`, `!=`).
fn find_boolean_comparisons(source: &str, skip: &[bool]) -> Vec<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut results = Vec::new();
    let mut i = 0;

    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Try to match `!=` or `<>` or `=` operators
        let (op_len, is_op) = if i + 1 < len && bytes[i] == b'!' && bytes[i + 1] == b'=' {
            (2, true)
        } else if i + 1 < len && bytes[i] == b'<' && bytes[i + 1] == b'>' {
            (2, true)
        } else if bytes[i] == b'=' {
            (1, true)
        } else {
            (0, false)
        };

        if is_op {
            let op_offset = i;
            // Advance past the operator
            let mut j = i + op_len;
            // Skip whitespace after operator
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                j += 1;
            }
            // Check for TRUE or FALSE (case-insensitive, word boundary)
            if j < len && !skip[j] {
                if bool_keyword_at(bytes, j, b"TRUE") || bool_keyword_at(bytes, j, b"FALSE") {
                    results.push(op_offset);
                }
            }
            i += op_len;
            continue;
        }

        i += 1;
    }

    results
}

impl Rule for BooleanComparison {
    fn name(&self) -> &'static str {
        "Convention/BooleanComparison"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        let bytes = source.as_bytes();
        let skip = build_skip(bytes);
        let offsets = find_boolean_comparisons(source, &skip);

        offsets
            .into_iter()
            .map(|op_offset| {
                let (line, col) = line_col(source, op_offset);
                Diagnostic {
                    rule: self.name(),
                    message: "Explicit comparison with boolean literal; use the expression directly"
                        .to_string(),
                    line,
                    col,
                }
            })
            .collect()
    }
}
