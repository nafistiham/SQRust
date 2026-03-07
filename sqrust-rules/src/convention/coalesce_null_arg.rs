use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct CoalesceNullArg;

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

/// Scans inside a COALESCE argument list (starting after the opening `(`)
/// for a bare `NULL` keyword at any argument position (outside nested
/// strings/parens). Returns `true` if a NULL argument is found.
///
/// The `skip` table covers the outer source context; inside the arg list we
/// track our own nesting depth to handle nested function calls correctly.
fn args_contain_null(bytes: &[u8], skip: &[bool], open_paren: usize) -> bool {
    let len = bytes.len();
    let mut i = open_paren + 1; // step past '('
    let mut depth = 1usize; // paren depth relative to COALESCE's opening paren

    while i < len && depth > 0 {
        // If the skip table marks this position as inside a string/comment in
        // the outer source context, skip it.
        if skip[i] {
            i += 1;
            continue;
        }

        match bytes[i] {
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
            }
            _ => {
                // At depth == 1 we are in the top-level COALESCE arg list.
                // Check for `NULL` keyword at word boundary (not inside a nested call).
                if depth == 1 && keyword_at(bytes, i, b"NULL") {
                    return true;
                }
                i += 1;
            }
        }
    }

    false
}

/// A violation found in the source.
struct Match {
    /// Byte offset of the `COALESCE` keyword.
    keyword_offset: usize,
}

/// Scans `source` for `COALESCE(...)` calls that contain a NULL literal
/// argument, outside strings/comments.
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

        // Match `COALESCE` keyword with word boundary
        if keyword_at(bytes, i, b"COALESCE") {
            let kw_offset = i;
            let mut j = i + 8; // past "COALESCE"

            // Skip whitespace between keyword and `(`
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                j += 1;
            }

            // Must be followed immediately by `(`
            if j < len && bytes[j] == b'(' && !skip[j] {
                if args_contain_null(bytes, skip, j) {
                    results.push(Match { keyword_offset: kw_offset });
                }
            }

            i += 8; // advance past "COALESCE"
            continue;
        }

        i += 1;
    }

    results
}

impl Rule for CoalesceNullArg {
    fn name(&self) -> &'static str {
        "Convention/CoalesceNullArg"
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
                    message: "NULL argument in COALESCE() is redundant; COALESCE skips NULL values automatically".to_string(),
                    line,
                    col,
                }
            })
            .collect()
    }
}
