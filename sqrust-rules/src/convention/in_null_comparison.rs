use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct InNullComparison;

/// Returns `true` if `bytes[offset..]` starts with `pattern` case-insensitively,
/// with a word boundary before and after.
fn keyword_at_boundary(bytes: &[u8], offset: usize, pattern: &[u8]) -> bool {
    let end = offset + pattern.len();
    if end > bytes.len() {
        return false;
    }
    // Word boundary before: preceding char must not be alphanumeric or underscore
    if offset > 0 && (bytes[offset - 1].is_ascii_alphanumeric() || bytes[offset - 1] == b'_') {
        return false;
    }
    // Case-insensitive match
    let matches = bytes[offset..end]
        .iter()
        .zip(pattern.iter())
        .all(|(&a, &b)| a.eq_ignore_ascii_case(&b));
    if !matches {
        return false;
    }
    // Word boundary after
    if end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        return false;
    }
    true
}

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

/// Describes one `IN (NULL)` or `NOT IN (NULL)` match.
struct Match {
    /// Byte offset of the `IN` keyword.
    in_offset: usize,
    /// Whether this was `NOT IN`.
    is_not_in: bool,
}

/// Scans `source` for `IN (NULL)` and `NOT IN (NULL)` patterns outside
/// strings/comments.
fn find_matches(source: &str, skip: &[bool]) -> Vec<Match> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut matches = Vec::new();
    let mut i = 0;

    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Try to match `NOT` keyword, then check for `IN` after it.
        // Also try plain `IN`.
        let is_not_in = keyword_at_boundary(bytes, i, b"NOT") && !skip[i];
        if is_not_in {
            // Skip NOT + whitespace to find IN
            let mut j = i + 3; // past "NOT"
            while j < len && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < len && !skip[j] && keyword_at_boundary(bytes, j, b"IN") {
                let in_offset = j;
                // Skip past IN + whitespace
                let mut k = j + 2;
                while k < len && bytes[k].is_ascii_whitespace() {
                    k += 1;
                }
                if k < len && bytes[k] == b'(' && !skip[k] {
                    if let Some(m) = check_paren_null(bytes, skip, k) {
                        if m {
                            matches.push(Match { in_offset, is_not_in: true });
                            i = k + 1;
                            continue;
                        }
                    }
                }
            }
        }

        // Try plain `IN` (word boundary, outside skip)
        if !skip[i] && keyword_at_boundary(bytes, i, b"IN") {
            let in_offset = i;
            let mut j = i + 2; // past "IN"
            while j < len && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < len && bytes[j] == b'(' && !skip[j] {
                if let Some(m) = check_paren_null(bytes, skip, j) {
                    if m {
                        matches.push(Match { in_offset, is_not_in: false });
                        i = j + 1;
                        continue;
                    }
                }
            }
        }

        i += 1;
    }

    matches
}

/// Given the position of `(`, checks whether the content between `(` and `)` is
/// exactly `NULL` (case-insensitive, possibly surrounded by whitespace), with no
/// other tokens. Returns `Some(true)` if it is, `Some(false)` if it is not, and
/// `None` if no closing `)` was found.
fn check_paren_null(bytes: &[u8], skip: &[bool], open_paren: usize) -> Option<bool> {
    let len = bytes.len();
    let mut i = open_paren + 1; // step past '('

    // Skip leading whitespace
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    // Expect exactly `NULL`
    if i + 4 > len {
        return Some(false);
    }
    let null_start = i;
    // Case-insensitive NULL check
    let is_null = bytes[null_start..null_start + 4]
        .iter()
        .zip(b"NULL".iter())
        .all(|(&a, &b)| a.eq_ignore_ascii_case(&b));
    if !is_null {
        return Some(false);
    }

    // Any of the NULL bytes in a skipped region means it's inside a string —
    // bail out.
    for k in null_start..null_start + 4 {
        if skip[k] {
            return Some(false);
        }
    }

    i = null_start + 4;

    // Word boundary: after NULL must not be alphanumeric or underscore
    if i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        return Some(false);
    }

    // Skip trailing whitespace
    while i < len && bytes[i].is_ascii_whitespace() {
        i += 1;
    }

    // Next must be `)`
    if i < len && bytes[i] == b')' && !skip[i] {
        Some(true)
    } else {
        Some(false)
    }
}

impl Rule for InNullComparison {
    fn name(&self) -> &'static str {
        "Convention/InNullComparison"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip(source.as_bytes());
        let matches = find_matches(source, &skip);

        matches
            .into_iter()
            .map(|m| {
                let (line, col) = line_col(source, m.in_offset);
                let message = if m.is_not_in {
                    "Use IS NOT NULL instead of NOT IN (NULL)".to_string()
                } else {
                    "Use IS NULL instead of IN (NULL)".to_string()
                };
                Diagnostic {
                    rule: self.name(),
                    message,
                    line,
                    col,
                }
            })
            .collect()
    }
}
