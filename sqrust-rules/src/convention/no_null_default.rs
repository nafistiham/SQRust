use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NoNullDefault;

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Returns `true` if `ch` is a SQL word character (`[a-zA-Z0-9_]`).
#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

#[inline]
fn is_whitespace(ch: u8) -> bool {
    ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r'
}

/// Builds a skip table: `true` at every byte inside strings or comments.
fn build_skip(bytes: &[u8]) -> Vec<bool> {
    let len = bytes.len();
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Line comment: -- ... end-of-line
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

/// Returns `true` if `bytes[pos..]` starts with `keyword` (case-insensitive)
/// and all matched bytes are code (not inside string/comment).
fn matches_keyword_at(bytes: &[u8], len: usize, skip: &[bool], pos: usize, keyword: &[u8]) -> bool {
    let kw_len = keyword.len();
    if pos + kw_len > len {
        return false;
    }
    (0..kw_len).all(|k| !skip[pos + k] && bytes[pos + k].eq_ignore_ascii_case(&keyword[k]))
}

/// Scans `source` for `DEFAULT NULL` patterns outside strings/comments.
/// Returns byte offsets of each `DEFAULT` keyword that is followed by `NULL`.
fn find_default_null_offsets(source: &str, skip: &[bool]) -> Vec<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut results = Vec::new();
    let mut i = 0;

    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Try to match DEFAULT at position i.
        if !matches_keyword_at(bytes, len, skip, i, b"DEFAULT") {
            i += 1;
            continue;
        }

        // Word boundary before DEFAULT.
        if i > 0 && is_word_char(bytes[i - 1]) {
            i += 1;
            continue;
        }

        let default_start = i;
        let default_end = i + 7; // len("DEFAULT") == 7

        // Word boundary after DEFAULT.
        if default_end < len && is_word_char(bytes[default_end]) {
            i += 1;
            continue;
        }

        // Skip whitespace between DEFAULT and what follows.
        let mut j = default_end;
        while j < len && !skip[j] && is_whitespace(bytes[j]) {
            j += 1;
        }

        // There must be at least one whitespace between DEFAULT and NULL.
        if j == default_end {
            i += 1;
            continue;
        }

        // Try to match NULL at position j.
        if !matches_keyword_at(bytes, len, skip, j, b"NULL") {
            i += 1;
            continue;
        }

        // Word boundary before NULL.
        if j > 0 && is_word_char(bytes[j - 1]) {
            i += 1;
            continue;
        }

        let null_end = j + 4; // len("NULL") == 4

        // Word boundary after NULL.
        if null_end < len && is_word_char(bytes[null_end]) {
            i += 1;
            continue;
        }

        results.push(default_start);
        i = null_end;
    }

    results
}

impl Rule for NoNullDefault {
    fn name(&self) -> &'static str {
        "Convention/NoNullDefault"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let skip = build_skip(bytes);
        let offsets = find_default_null_offsets(source, &skip);

        offsets
            .into_iter()
            .map(|offset| {
                let (line, col) = line_col(source, offset);
                Diagnostic {
                    rule: self.name(),
                    message: "DEFAULT NULL is redundant; omit it to use the implicit default"
                        .to_string(),
                    line,
                    col,
                }
            })
            .collect()
    }
}
