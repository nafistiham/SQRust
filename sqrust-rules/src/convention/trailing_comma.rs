use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct TrailingComma;

/// Returns `true` if `source[offset..]` starts with `pattern` case-insensitively,
/// and the character after the pattern is a word boundary (not `[a-zA-Z0-9_]`).
fn keyword_at(bytes: &[u8], offset: usize, pattern: &[u8]) -> bool {
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
    // word boundary: next char must not be alphanumeric or underscore
    if end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        return false;
    }
    true
}

/// The SQL keywords that terminate a SELECT list. Checked in order; longest
/// first so `INTERSECT` is tried before `IN` (though `IN` is not in this list).
const TERMINATORS: &[&[u8]] = &[
    b"INTERSECT",
    b"EXCEPT",
    b"HAVING",
    b"UNION",
    b"GROUP",
    b"ORDER",
    b"WHERE",
    b"LIMIT",
    b"FROM",
];

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip table: `true` at every byte that is inside a string literal,
/// line comment, block comment, or quoted identifier.
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

/// Finds all trailing-comma byte offsets: a `,` (outside strings/comments) whose
/// next non-whitespace token starts with a terminator keyword.
fn find_trailing_commas(source: &str, skip: &[bool]) -> Vec<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut positions = Vec::new();
    let mut i = 0;

    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        if bytes[i] == b',' {
            // Look ahead past whitespace
            let comma_offset = i;
            let mut j = i + 1;
            while j < len && bytes[j].is_ascii_whitespace() {
                j += 1;
            }

            // Check if next non-whitespace is a terminator keyword
            for &kw in TERMINATORS {
                if keyword_at(bytes, j, kw) {
                    positions.push(comma_offset);
                    break;
                }
            }
        }

        i += 1;
    }

    positions
}

impl Rule for TrailingComma {
    fn name(&self) -> &'static str {
        "Convention/TrailingComma"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let skip = build_skip(source.as_bytes());
        let positions = find_trailing_commas(source, &skip);

        positions
            .into_iter()
            .map(|offset| {
                let (line, col) = line_col(source, offset);
                Diagnostic {
                    rule: self.name(),
                    message: "Trailing comma before SQL keyword".to_string(),
                    line,
                    col,
                }
            })
            .collect()
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let source = &ctx.source;
        let skip = build_skip(source.as_bytes());
        let positions = find_trailing_commas(source, &skip);

        if positions.is_empty() {
            return None;
        }

        // Remove commas in reverse order so earlier offsets stay valid
        let mut result = source.clone();
        for offset in positions.into_iter().rev() {
            result.remove(offset);
        }

        Some(result)
    }
}
