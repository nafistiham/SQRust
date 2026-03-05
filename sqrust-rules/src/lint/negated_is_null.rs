use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct NegatedIsNull;

impl Rule for NegatedIsNull {
    fn name(&self) -> &'static str {
        "Lint/NegatedIsNull"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = build_skip(bytes);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Skip positions inside strings/comments.
            if skip[i] {
                i += 1;
                continue;
            }

            // Check for `NOT` keyword (case-insensitive, word boundary).
            if !is_word_boundary_before(bytes, i)
                || !starts_with_ci(bytes, i, b"NOT")
                || is_word_char_at(bytes, i + 3)
            {
                i += 1;
                continue;
            }

            // Found `NOT`. Record its position and advance past it.
            let not_start = i;
            let mut j = i + 3; // past "NOT"

            // Skip whitespace (including zero whitespace — `NOT(` is valid).
            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                j += 1;
            }

            // Optionally consume `(`.
            let mut had_paren = false;
            if j < len && bytes[j] == b'(' && !skip[j] {
                had_paren = true;
                j += 1;
            }

            // Skip whitespace after optional paren.
            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                j += 1;
            }

            // Try to match a column reference: word chars and dots (e.g. `col` or `t.col`).
            if j >= len || !is_word_char_at(bytes, j) {
                i += 1;
                continue;
            }
            while j < len && (is_word_char_at(bytes, j) || (bytes[j] == b'.' && !skip[j])) {
                j += 1;
            }

            // Skip whitespace.
            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                j += 1;
            }

            // Match `IS` keyword.
            if !starts_with_ci(bytes, j, b"IS") || is_word_char_at(bytes, j + 2) || skip[j] {
                i += 1;
                continue;
            }
            j += 2;

            // Require whitespace after `IS`.
            if j >= len || !bytes[j].is_ascii_whitespace() {
                i += 1;
                continue;
            }
            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                j += 1;
            }

            // Match `NULL` keyword — but NOT if followed by a space and then `NOT`
            // (i.e. `IS NOT NULL` — the already-correct form). We want `IS NULL` only.
            if !starts_with_ci(bytes, j, b"NULL") || skip[j] {
                i += 1;
                continue;
            }
            // Make sure it really is `NULL` and not `NOT` preceded by `IS ` (already handled above).
            // Also make sure `NOT` doesn't appear between `IS` and `NULL` (that would be IS NOT NULL).
            // Since we already matched whitespace + "NULL" directly, this is the `IS NULL` sub-pattern.
            if is_word_char_at(bytes, j + 4) {
                i += 1;
                continue;
            }

            // We have `NOT [whitespace] [(] [whitespace] <col> [whitespace] IS [whitespace] NULL`.
            // Emit diagnostic at the `NOT` position.
            let (line, col) = line_col(source, not_start);
            diags.push(Diagnostic {
                rule: self.name(),
                message: "Use IS NOT NULL instead of NOT ... IS NULL".to_string(),
                line,
                col,
            });

            // Advance past the full match so we don't re-scan inside it.
            // Close paren will be consumed naturally; just move past `NULL`.
            i = j + 4;
            // If there was a paren, skip the closing `)` if present.
            if had_paren {
                // Skip optional whitespace then optional `)`.
                while i < len && bytes[i].is_ascii_whitespace() && !skip[i] {
                    i += 1;
                }
                if i < len && bytes[i] == b')' && !skip[i] {
                    i += 1;
                }
            }
            continue;
        }

        diags
    }

    fn fix(&self, ctx: &FileContext) -> Option<String> {
        let source = &ctx.source;
        let diags = self.check(ctx);
        if diags.is_empty() {
            return None;
        }

        // For each violation, find and replace the pattern in source.
        // We collect replacements and apply them in reverse order so offsets stay valid.
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = build_skip(bytes);

        let mut replacements: Vec<(usize, usize, String)> = Vec::new();
        let mut i = 0;

        while i < len {
            if skip[i] {
                i += 1;
                continue;
            }

            if !is_word_boundary_before(bytes, i)
                || !starts_with_ci(bytes, i, b"NOT")
                || is_word_char_at(bytes, i + 3)
            {
                i += 1;
                continue;
            }

            let pattern_start = i;
            let mut j = i + 3;

            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                j += 1;
            }

            let mut had_paren = false;
            if j < len && bytes[j] == b'(' && !skip[j] {
                had_paren = true;
                j += 1;
            }

            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                j += 1;
            }

            if j >= len || !is_word_char_at(bytes, j) {
                i += 1;
                continue;
            }

            let col_start = j;
            while j < len && (is_word_char_at(bytes, j) || (bytes[j] == b'.' && !skip[j])) {
                j += 1;
            }
            let col_end = j;
            let col_name = &source[col_start..col_end];

            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                j += 1;
            }

            if !starts_with_ci(bytes, j, b"IS") || is_word_char_at(bytes, j + 2) || skip[j] {
                i += 1;
                continue;
            }
            j += 2;

            if j >= len || !bytes[j].is_ascii_whitespace() {
                i += 1;
                continue;
            }
            while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                j += 1;
            }

            if !starts_with_ci(bytes, j, b"NULL") || skip[j] || is_word_char_at(bytes, j + 4) {
                i += 1;
                continue;
            }
            j += 4;

            // Capture the closing `)` if we had an opening paren.
            if had_paren {
                let mut k = j;
                while k < len && bytes[k].is_ascii_whitespace() && !skip[k] {
                    k += 1;
                }
                if k < len && bytes[k] == b')' && !skip[k] {
                    j = k + 1;
                }
            }

            let replacement = format!("{} IS NOT NULL", col_name);
            replacements.push((pattern_start, j, replacement));
            i = j;
            continue;
        }

        if replacements.is_empty() {
            return None;
        }

        let mut result = source.clone();
        for (start, end, rep) in replacements.into_iter().rev() {
            result.replace_range(start..end, &rep);
        }
        Some(result)
    }
}

/// Returns `true` if `ch` is a word character (`[a-zA-Z0-9_]`).
#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Returns `true` if `bytes[pos]` is a word character (returns false if out of bounds).
#[inline]
fn is_word_char_at(bytes: &[u8], pos: usize) -> bool {
    pos < bytes.len() && is_word_char(bytes[pos])
}

/// Returns `true` if position `i` is a word boundary start — i.e. the byte
/// before `i` is NOT a word character (or `i` is 0).
#[inline]
fn is_word_boundary_before(bytes: &[u8], i: usize) -> bool {
    i == 0 || !is_word_char(bytes[i - 1])
}

/// Returns `true` if `bytes[offset..]` starts with `pattern`, case-insensitively (ASCII).
fn starts_with_ci(bytes: &[u8], offset: usize, pattern: &[u8]) -> bool {
    let end = offset + pattern.len();
    if end > bytes.len() {
        return false;
    }
    bytes[offset..end]
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

/// Builds a skip table: `true` for every byte offset inside a string literal,
/// line comment, block comment, or quoted identifier.
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

        // Single-quoted string: '...' with '' as escaped quote
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
