use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct IsNull;

/// Returns `true` if `source[offset..]` starts with `pattern`,
/// compared case-insensitively for ASCII characters.
fn starts_with_ci(source: &[u8], offset: usize, pattern: &[u8]) -> bool {
    let end = offset + pattern.len();
    if end > source.len() {
        return false;
    }
    source[offset..end]
        .iter()
        .zip(pattern.iter())
        .all(|(&a, &b)| a.eq_ignore_ascii_case(&b))
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip table: each entry is `true` if the byte at that offset is
/// inside a string literal, line comment, block comment, or quoted identifier.
fn build_skip(source: &[u8]) -> Vec<bool> {
    let len = source.len();
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Line comment: -- ... end-of-line
        if i + 1 < len && source[i] == b'-' && source[i + 1] == b'-' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len && source[i] != b'\n' {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && source[i] == b'/' && source[i + 1] == b'*' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len {
                if i + 1 < len && source[i] == b'*' && source[i + 1] == b'/' {
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
        if source[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                if source[i] == b'\'' {
                    skip[i] = true;
                    i += 1;
                    if i < len && source[i] == b'\'' {
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
        if source[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len && source[i] != b'"' {
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
        if source[i] == b'`' {
            skip[i] = true;
            i += 1;
            while i < len && source[i] != b'`' {
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

/// A detected null-comparison pattern with metadata for diagnostics and fix.
struct NullMatch {
    /// Byte offset of the operator (`=`, `<>`, or `!=`)
    op_offset: usize,
    /// Length of the full matched span (operator + spaces + NULL)
    full_len: usize,
    /// Replacement text (e.g. `IS NULL` or `IS NOT NULL`)
    replacement: &'static str,
    /// Diagnostic message
    message: &'static str,
}

/// Tries to match `NULL` (case-insensitive) after skipping whitespace from
/// byte index `after_op`. Returns `Some(NullMatch)` on success.
fn try_match_null(
    bytes: &[u8],
    skip: &[bool],
    op_start: usize,
    op_len: usize,
    replacement: &'static str,
    message: &'static str,
) -> Option<NullMatch> {
    let len = bytes.len();
    let mut j = op_start + op_len;

    // Require at least one whitespace between operator and NULL
    if j >= len || !bytes[j].is_ascii_whitespace() {
        return None;
    }

    // Skip whitespace — all must be code (outside strings/comments)
    while j < len && bytes[j].is_ascii_whitespace() {
        if skip[j] {
            return None;
        }
        j += 1;
    }

    // Check for NULL (case-insensitive) as a word boundary
    if j + 4 > len {
        return None;
    }
    if !starts_with_ci(bytes, j, b"NULL") {
        return None;
    }

    // Make sure the 4 NULL bytes are all outside strings/comments
    for k in j..j + 4 {
        if skip[k] {
            return None;
        }
    }

    // Word boundary after NULL: must not be followed by `[a-zA-Z0-9_]`
    let end = j + 4;
    if end < len && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        return None;
    }

    Some(NullMatch {
        op_offset: op_start,
        full_len: end - op_start,
        replacement,
        message,
    })
}

/// Scans `source` for `= NULL`, `<> NULL`, `!= NULL` outside strings/comments.
fn find_null_matches(source: &str, skip: &[bool]) -> Vec<NullMatch> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut matches = Vec::new();
    let mut i = 0;

    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        // `= NULL` — but not when preceded by `!` (!=) or `<` (<>)
        if bytes[i] == b'=' {
            let preceded_by_bang = i > 0 && bytes[i - 1] == b'!';
            let preceded_by_lt = i > 0 && bytes[i - 1] == b'<';
            if !preceded_by_bang && !preceded_by_lt {
                if let Some(m) = try_match_null(
                    bytes,
                    skip,
                    i,
                    1,
                    "IS NULL",
                    "Use IS NULL instead of = NULL",
                ) {
                    matches.push(m);
                    i += 1;
                    continue;
                }
            }
        }

        // `<> NULL`
        if bytes[i] == b'<' && i + 1 < len && bytes[i + 1] == b'>' {
            if let Some(m) = try_match_null(
                bytes,
                skip,
                i,
                2,
                "IS NOT NULL",
                "Use IS NOT NULL instead of <> NULL",
            ) {
                matches.push(m);
                i += 2;
                continue;
            }
        }

        // `!= NULL`
        if bytes[i] == b'!' && i + 1 < len && bytes[i + 1] == b'=' {
            if let Some(m) = try_match_null(
                bytes,
                skip,
                i,
                2,
                "IS NOT NULL",
                "Use IS NOT NULL instead of != NULL",
            ) {
                matches.push(m);
                i += 2;
                continue;
            }
        }

        i += 1;
    }

    matches
}

impl Rule for IsNull {
    fn name(&self) -> &'static str {
        "Convention/IsNull"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip(source.as_bytes());
        let matches = find_null_matches(source, &skip);

        matches
            .into_iter()
            .map(|m| {
                let (line, col) = line_col(source, m.op_offset);
                Diagnostic {
                    rule: self.name(),
                    message: m.message.to_string(),
                    line,
                    col,
                }
            })
            .collect()
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let source = &ctx.source;
        let skip = build_skip(source.as_bytes());
        let matches = find_null_matches(source, &skip);

        if matches.is_empty() {
            return None;
        }

        // Apply replacements in reverse order to keep earlier byte offsets valid
        let mut result = source.clone();
        for m in matches.into_iter().rev() {
            result.replace_range(m.op_offset..m.op_offset + m.full_len, m.replacement);
        }

        Some(result)
    }
}
