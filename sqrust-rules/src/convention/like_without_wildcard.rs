use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct LikeWithoutWildcard;

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

/// Extracts the content of a single-quoted string starting at `open_quote`.
/// Returns the string content (without quotes), or None if not a valid
/// single-quoted string at that position.
///
/// The skip table marks the opening `'` as skip=true (it starts a string
/// region), but the byte itself is still at `open_quote` in `bytes`.
fn extract_single_quoted(bytes: &[u8], open_quote: usize) -> Option<Vec<u8>> {
    if open_quote >= bytes.len() || bytes[open_quote] != b'\'' {
        return None;
    }
    let mut content = Vec::new();
    let mut i = open_quote + 1;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            // '' is an escaped single quote
            if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                content.push(b'\'');
                i += 2;
                continue;
            }
            // Closing quote
            return Some(content);
        }
        content.push(bytes[i]);
        i += 1;
    }
    None // unterminated string
}

/// A violation found in the source.
struct Match {
    /// Byte offset of the `LIKE` (or `ILIKE`) keyword.
    like_offset: usize,
}

/// Scans `source` for `LIKE 'literal'` and `ILIKE 'literal'` patterns
/// (including `NOT LIKE`) outside strings/comments, where the pattern string
/// contains no wildcard characters (`%` or `_`).
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

        // Check for LIKE or ILIKE keyword
        let (is_like, kw_len) = if keyword_at(bytes, i, b"LIKE") {
            (true, 4)
        } else if keyword_at(bytes, i, b"ILIKE") {
            (true, 5)
        } else {
            (false, 0)
        };

        if is_like {
            let like_offset = i;
            let mut j = i + kw_len;

            // Skip whitespace after the keyword
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                j += 1;
            }

            // The pattern must be a single-quoted literal.
            // Note: the opening `'` byte is marked skip=true by build_skip
            // because it starts a string region, but we still read the byte
            // directly to detect whether a literal follows.
            if j < len && bytes[j] == b'\'' {
                if let Some(content) = extract_single_quoted(bytes, j) {
                    // Flag if the pattern contains no wildcard character
                    let has_wildcard = content.contains(&b'%') || content.contains(&b'_');
                    if !has_wildcard {
                        results.push(Match { like_offset });
                    }
                }
            }
            // Whether we flagged or not, advance past the keyword.
            i += kw_len;
            continue;
        }

        i += 1;
    }

    results
}

impl Rule for LikeWithoutWildcard {
    fn name(&self) -> &'static str {
        "Convention/LikeWithoutWildcard"
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
                Diagnostic {
                    rule: self.name(),
                    message: "LIKE with no wildcard characters; use = instead".to_string(),
                    line,
                    col,
                }
            })
            .collect()
    }
}
