use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct WhereTautology;

impl Rule for WhereTautology {
    fn name(&self) -> &'static str {
        "Lint/WhereTautology"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        let skip = SkipSet::build(source);
        let upper = source.to_uppercase();
        let bytes = upper.as_bytes();
        let len = bytes.len();

        let mut diags = Vec::new();

        // Patterns to scan for (in uppercased text):
        //   WHERE\s+1\s*=\s*1\b
        //   WHERE\s+TRUE\b
        //   AND\s+1\s*=\s*1\b
        //   AND\s+TRUE\b
        let mut i = 0usize;
        while i < len {
            // Skip positions inside comments or string literals.
            if skip.is_skip(i) {
                i += 1;
                continue;
            }

            // Try to match one of the tautology patterns starting at i.
            if let Some(end) = match_where_tautology(bytes, i, len) {
                let (line, col) = offset_to_line_col(source, i);
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "Tautological WHERE condition always evaluates to true".to_string(),
                    line,
                    col,
                });
                // Advance past the matched pattern to avoid re-matching.
                i = end;
                continue;
            }

            i += 1;
        }

        diags
    }
}

/// Returns Some(end_offset) if a tautology pattern starts at `pos`, None otherwise.
/// Patterns (case-insensitive, already uppercased in `bytes`):
///   WHERE\s+1\s*=\s*1\b
///   WHERE\s+TRUE\b
///   AND\s+1\s*=\s*1\b
///   AND\s+TRUE\b
fn match_where_tautology(bytes: &[u8], pos: usize, len: usize) -> Option<usize> {
    // Try WHERE patterns.
    if let Some(after_kw) = match_keyword(bytes, pos, len, b"WHERE") {
        if let Some(end) = match_tautology_body(bytes, after_kw, len) {
            return Some(end);
        }
    }
    // Try AND patterns.
    if let Some(after_kw) = match_keyword(bytes, pos, len, b"AND") {
        if let Some(end) = match_tautology_body(bytes, after_kw, len) {
            return Some(end);
        }
    }
    None
}

/// Checks if the keyword `kw` starts at `pos` in `bytes` with word boundaries.
/// Returns Some(pos_after_keyword) if matched, None otherwise.
fn match_keyword(bytes: &[u8], pos: usize, len: usize, kw: &[u8]) -> Option<usize> {
    let kw_len = kw.len();
    if pos + kw_len > len {
        return None;
    }
    if &bytes[pos..pos + kw_len] != kw {
        return None;
    }
    // Check word boundary before.
    let before_ok = pos == 0 || {
        let b = bytes[pos - 1];
        !b.is_ascii_alphanumeric() && b != b'_'
    };
    // Check word boundary after.
    let after_pos = pos + kw_len;
    let after_ok = after_pos >= len || {
        let b = bytes[after_pos];
        !b.is_ascii_alphanumeric() && b != b'_'
    };
    if before_ok && after_ok {
        Some(after_pos)
    } else {
        None
    }
}

/// After the WHERE/AND keyword, expect:
///   \s+1\s*=\s*1\b  or  \s+TRUE\b
/// Returns Some(end_offset) or None.
fn match_tautology_body(bytes: &[u8], pos: usize, len: usize) -> Option<usize> {
    // Must have at least one whitespace after keyword.
    let pos = skip_required_whitespace(bytes, pos, len)?;

    // Try `1\s*=\s*1\b`
    if pos < len && bytes[pos] == b'1' {
        let pos2 = skip_optional_whitespace(bytes, pos + 1, len);
        if pos2 < len && bytes[pos2] == b'=' {
            let pos3 = skip_optional_whitespace(bytes, pos2 + 1, len);
            if pos3 < len && bytes[pos3] == b'1' {
                let after = pos3 + 1;
                // Word boundary after the trailing '1'.
                let boundary_ok = after >= len || {
                    let b = bytes[after];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
                if boundary_ok {
                    return Some(after);
                }
            }
        }
    }

    // Try `TRUE\b`
    if pos + 4 <= len && &bytes[pos..pos + 4] == b"TRUE" {
        let after = pos + 4;
        let boundary_ok = after >= len || {
            let b = bytes[after];
            !b.is_ascii_alphanumeric() && b != b'_'
        };
        if boundary_ok {
            return Some(after);
        }
    }

    None
}

/// Skip one or more ASCII whitespace characters. Returns Some(new_pos) if at
/// least one whitespace was consumed, None otherwise.
fn skip_required_whitespace(bytes: &[u8], mut pos: usize, len: usize) -> Option<usize> {
    if pos >= len || !bytes[pos].is_ascii_whitespace() {
        return None;
    }
    while pos < len && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    Some(pos)
}

/// Skip zero or more ASCII whitespace characters. Always returns new_pos.
fn skip_optional_whitespace(bytes: &[u8], mut pos: usize, len: usize) -> usize {
    while pos < len && bytes[pos].is_ascii_whitespace() {
        pos += 1;
    }
    pos
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Tracks byte positions that are inside single-quoted strings, double-quoted
/// identifiers, `-- ...` line comments, or `/* ... */` block comments.
struct SkipSet {
    skip: Vec<bool>,
}

impl SkipSet {
    fn build(source: &str) -> Self {
        let bytes = source.as_bytes();
        let len = bytes.len();
        let mut skip = vec![false; len];

        let mut i = 0usize;
        while i < len {
            // Block comment: /* ... */
            if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
                let start = i;
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                let end = if i + 1 < len { i + 2 } else { i + 1 };
                for s in &mut skip[start..end.min(len)] {
                    *s = true;
                }
                i = end;
                continue;
            }
            // Line comment: -- ...
            if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
                let start = i;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                for s in &mut skip[start..i] {
                    *s = true;
                }
                continue; // don't advance — newline itself is code
            }
            // Single-quoted string: '...' ('' is an escaped quote)
            if bytes[i] == b'\'' {
                let start = i;
                i += 1;
                while i < len {
                    if bytes[i] == b'\'' {
                        if i + 1 < len && bytes[i + 1] == b'\'' {
                            i += 2; // escaped quote
                        } else {
                            i += 1; // closing quote
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                for s in &mut skip[start..i.min(len)] {
                    *s = true;
                }
                continue;
            }
            // Double-quoted identifier: "..."
            if bytes[i] == b'"' {
                let start = i;
                i += 1;
                while i < len && bytes[i] != b'"' {
                    i += 1;
                }
                let end = if i < len { i + 1 } else { i };
                for s in &mut skip[start..end.min(len)] {
                    *s = true;
                }
                i = end;
                continue;
            }
            i += 1;
        }

        SkipSet { skip }
    }

    fn is_skip(&self, pos: usize) -> bool {
        self.skip.get(pos).copied().unwrap_or(false)
    }
}
