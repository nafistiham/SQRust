use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct LikePercentOnly;

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
/// with word boundary checks before (caller ensures) and after.
fn keyword_at(bytes: &[u8], offset: usize, pattern: &[u8]) -> bool {
    let end = offset + pattern.len();
    if end > bytes.len() {
        return false;
    }
    // Word boundary before
    if offset > 0 && (bytes[offset - 1].is_ascii_alphanumeric() || bytes[offset - 1] == b'_') {
        return false;
    }
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

/// Describes one match found in the source.
struct Match {
    /// Byte offset of the `LIKE` keyword.
    like_offset: usize,
    /// Whether `NOT LIKE` was matched.
    is_not_like: bool,
}

/// Scans `source` for `LIKE '%'` and `NOT LIKE '%'` outside strings/comments.
/// The pattern `'%'` must be exactly one percent sign.
fn find_matches(source: &str, skip: &[bool]) -> Vec<Match> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut results = Vec::new();
    let mut i = 0;

    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Check for `NOT` keyword
        if keyword_at(bytes, i, b"NOT") {
            let mut j = i + 3; // past "NOT"
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            // LIKE keyword must not be inside a skipped region
            if j < len && !skip[j] && keyword_at(bytes, j, b"LIKE") {
                let like_offset = j;
                let mut k = j + 4; // past "LIKE"
                while k < len && (bytes[k] == b' ' || bytes[k] == b'\t') {
                    k += 1;
                }
                // Do not guard with skip[k] here: the opening `'` of the LIKE
                // argument is marked as skip=true by build_skip (it starts a
                // string), but it is not inside a comment or prior string.
                if k < len {
                    if let Some(()) = check_single_percent_pattern(bytes, k) {
                        results.push(Match { like_offset, is_not_like: true });
                        i = k + 1;
                        continue;
                    }
                }
            }
        }

        // Check for plain `LIKE` keyword
        if keyword_at(bytes, i, b"LIKE") {
            let like_offset = i;
            let mut j = i + 4; // past "LIKE"
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            // Same reasoning: don't guard with skip[j] for the opening quote.
            if j < len {
                if let Some(()) = check_single_percent_pattern(bytes, j) {
                    results.push(Match { like_offset, is_not_like: false });
                    i = j + 1;
                    continue;
                }
            }
        }

        i += 1;
    }

    results
}

/// Given the position of `'`, checks whether the content is exactly `%`
/// (a single percent sign, nothing else inside the quotes).
/// Returns `Some(())` if matched, `None` otherwise.
///
/// Note: does not consult the skip table because the opening quote of a LIKE
/// argument is legitimately marked as a string delimiter by build_skip.
fn check_single_percent_pattern(bytes: &[u8], open_quote: usize) -> Option<()> {
    if bytes[open_quote] != b'\'' {
        return None;
    }
    // After opening quote there must be exactly one `%` then a closing `'`
    let percent_pos = open_quote + 1;
    let close_pos = open_quote + 2;
    if close_pos >= bytes.len() {
        return None;
    }
    if bytes[percent_pos] == b'%' && bytes[close_pos] == b'\'' {
        return Some(());
    }
    None
}

impl Rule for LikePercentOnly {
    fn name(&self) -> &'static str {
        "Convention/LikePercentOnly"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        let bytes = source.as_bytes();
        let skip = build_skip(bytes);
        let matches = find_matches(source, &skip);

        matches
            .into_iter()
            .map(|m| {
                let (line, col) = line_col(source, m.like_offset);
                let message = if m.is_not_like {
                    "NOT LIKE '%' matches nothing; use IS NULL instead".to_string()
                } else {
                    "LIKE '%' matches everything; use IS NOT NULL instead".to_string()
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
