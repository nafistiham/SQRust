use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct LikeTautology;

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

/// Returns true if `bytes[offset..]` starts with `keyword` case-insensitively
/// and is surrounded by word boundaries.
fn keyword_at(bytes: &[u8], offset: usize, keyword: &[u8]) -> bool {
    let end = offset + keyword.len();
    if end > bytes.len() {
        return false;
    }
    // Word boundary before
    if offset > 0 && (bytes[offset - 1].is_ascii_alphanumeric() || bytes[offset - 1] == b'_') {
        return false;
    }
    // Case-insensitive match
    let matches = bytes[offset..end]
        .iter()
        .zip(keyword.iter())
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

/// Given the position of `'`, checks whether the content is a pure-`%` pattern
/// (one or more `%` characters, nothing else). Returns `Some(())` if matched,
/// `None` otherwise.
///
/// Note: does not consult the skip table because the opening quote of a LIKE
/// argument is legitimately marked as a string delimiter by build_skip.
fn is_pure_percent_pattern(bytes: &[u8], open_quote: usize) -> bool {
    if open_quote >= bytes.len() || bytes[open_quote] != b'\'' {
        return false;
    }
    let mut i = open_quote + 1;
    if i >= bytes.len() {
        return false;
    }
    // Must have at least one `%`
    if bytes[i] != b'%' {
        return false;
    }
    // All subsequent characters must be `%` until the closing `'`
    while i < bytes.len() {
        match bytes[i] {
            b'%' => i += 1,
            b'\'' => return true, // closing quote — pattern is pure `%`
            _ => return false,    // non-`%` character found
        }
    }
    false // unterminated string
}

/// A violation found in the source.
struct Match {
    /// Byte offset of the `LIKE` or `ILIKE` keyword.
    keyword_offset: usize,
}

/// Scans `source` for `[NOT] LIKE '%%...'` and `ILIKE '%%...'` outside
/// strings/comments. `NOT LIKE '%'` is intentionally excluded — it matches
/// nothing and is a separate concern.
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

        // Check for `NOT` keyword — if followed by LIKE '%%', skip it (NOT LIKE
        // '%' is not a tautology).
        if keyword_at(bytes, i, b"NOT") {
            let mut j = i + 3; // past "NOT"
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                j += 1;
            }
            if j < len && !skip[j] && keyword_at(bytes, j, b"LIKE") {
                // This is NOT LIKE — skip it entirely (do not flag)
                i = j + 4;
                continue;
            }
        }

        // Check for ILIKE keyword (case-insensitive LIKE — also a tautology with '%')
        if keyword_at(bytes, i, b"ILIKE") {
            let kw_offset = i;
            let mut j = i + 5; // past "ILIKE"
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                j += 1;
            }
            // Opening `'` is marked skip=true (it starts a string region) but
            // the byte itself is still readable at j.
            if j < len && is_pure_percent_pattern(bytes, j) {
                results.push(Match { keyword_offset: kw_offset });
            }
            i += 5;
            continue;
        }

        // Check for plain LIKE keyword
        if keyword_at(bytes, i, b"LIKE") {
            let kw_offset = i;
            let mut j = i + 4; // past "LIKE"
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                j += 1;
            }
            if j < len && is_pure_percent_pattern(bytes, j) {
                results.push(Match { keyword_offset: kw_offset });
            }
            i += 4;
            continue;
        }

        i += 1;
    }

    results
}

impl Rule for LikeTautology {
    fn name(&self) -> &'static str {
        "Convention/LikeTautology"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        let skip = build_skip(source.as_bytes());
        let matches = find_matches(source, &skip);

        matches
            .into_iter()
            .map(|m| {
                let (line, col) = line_col(source, m.keyword_offset);
                Diagnostic {
                    rule: self.name(),
                    message: "LIKE '%' matches everything and is a no-op filter; remove it or use a meaningful pattern".to_string(),
                    line,
                    col,
                }
            })
            .collect()
    }
}
