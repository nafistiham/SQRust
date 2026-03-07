use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct EmptyInList;

impl Rule for EmptyInList {
    fn name(&self) -> &'static str {
        "Lint/EmptyInList"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = build_skip(bytes);

        // Uppercase copy for case-insensitive scanning.
        let upper: Vec<u8> = bytes.iter().map(|b| b.to_ascii_uppercase()).collect();

        let mut diags = Vec::new();
        let mut i = 0usize;

        while i < len {
            // Skip positions inside strings or comments.
            if skip[i] {
                i += 1;
                continue;
            }

            // Try to match the keyword `IN` at position i.
            // We need a word boundary before and after.
            if let Some(after_in) = match_keyword_at(&upper, &skip, i, len, b"IN") {
                // After `IN`, skip optional whitespace outside skip regions.
                let mut j = after_in;
                while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                    j += 1;
                }

                // Expect `(` outside a skip region.
                if j < len && bytes[j] == b'(' && !skip[j] {
                    let open_paren = j;
                    j += 1;

                    // Skip optional whitespace inside the parens (outside skip regions).
                    while j < len && bytes[j].is_ascii_whitespace() && !skip[j] {
                        j += 1;
                    }

                    // If the very next non-whitespace char is `)`, the list is empty.
                    if j < len && bytes[j] == b')' && !skip[j] {
                        // Report at the position of the `IN` keyword.
                        let _ = open_paren; // suppress unused warning
                        let (line, col) = offset_to_line_col(source, i);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: "Empty IN list always evaluates to FALSE".to_string(),
                            line,
                            col,
                        });
                        // Advance past `)` to avoid re-matching.
                        i = j + 1;
                        continue;
                    }
                }

                // Not empty IN — advance past the keyword.
                i = after_in;
                continue;
            }

            i += 1;
        }

        diags
    }
}

/// Returns `Some(pos_after_keyword)` if `kw` matches at `pos` in `upper`
/// (case-insensitive via the pre-uppercased `upper` slice), with word
/// boundaries on both sides and the position not inside a skip region.
/// Returns `None` otherwise.
fn match_keyword_at(
    upper: &[u8],
    skip: &[bool],
    pos: usize,
    len: usize,
    kw: &[u8],
) -> Option<usize> {
    let kw_len = kw.len();
    if pos + kw_len > len {
        return None;
    }
    // Must be outside a skip region.
    if skip[pos] {
        return None;
    }
    // Bytes must match (upper already uppercased).
    if &upper[pos..pos + kw_len] != kw {
        return None;
    }
    // Word boundary before.
    let before_ok = pos == 0 || {
        let b = upper[pos - 1];
        !b.is_ascii_alphanumeric() && b != b'_'
    };
    // Word boundary after.
    let after_pos = pos + kw_len;
    let after_ok = after_pos >= len || {
        let b = upper[after_pos];
        !b.is_ascii_alphanumeric() && b != b'_'
    };
    if before_ok && after_ok {
        Some(after_pos)
    } else {
        None
    }
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Builds a skip table: `true` for every byte offset that is inside a
/// string literal, line comment, block comment, or quoted identifier.
fn build_skip(bytes: &[u8]) -> Vec<bool> {
    let len = bytes.len();
    let mut skip = vec![false; len];
    let mut i = 0usize;

    while i < len {
        // Line comment: -- ... end-of-line
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            let start = i;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            for s in &mut skip[start..i] {
                *s = true;
            }
            // Don't advance — newline itself is not skipped.
            continue;
        }

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

        // Single-quoted string: '...' with '' as escaped quote
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

        // Backtick identifier: `...`
        if bytes[i] == b'`' {
            let start = i;
            i += 1;
            while i < len && bytes[i] != b'`' {
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

    skip
}
