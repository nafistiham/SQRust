use sqrust_core::{Diagnostic, FileContext, Rule};

/// Flag window function frame clauses that span the entire partition
/// unnecessarily: `ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING`
/// or `RANGE BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING`.
///
/// This is equivalent to no frame clause at all for aggregate functions but
/// forces the database to buffer all rows and is rarely intentional.
pub struct WindowFrameFullPartition;

impl Rule for WindowFrameFullPartition {
    fn name(&self) -> &'static str {
        "Structure/WindowFrameFullPartition"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(bytes, len);
    let mut diags = Vec::new();
    let mut i = 0;

    // Pattern: (ROWS|RANGE) BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING
    // We scan for BETWEEN, then validate the full pattern.
    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Look for BETWEEN keyword.
        if let Some(between_end) = match_keyword(bytes, len, &skip, i, b"BETWEEN") {
            // Scan backward (skipping whitespace) to find ROWS or RANGE.
            let has_rows_or_range = find_rows_or_range_before(bytes, &skip, i);

            if has_rows_or_range {
                // Now scan forward from between_end for:
                // UNBOUNDED <ws> PRECEDING <ws> AND <ws> UNBOUNDED <ws> FOLLOWING
                if let Some(_end) = match_unbounded_preceding_and_following(bytes, len, &skip, between_end) {
                    let (line, col) = offset_to_line_col(source, i);
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message: "ROWS/RANGE BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING spans the entire partition; this is rarely intentional and may indicate a logic error".to_string(),
                        line,
                        col,
                    });
                    i = _end;
                    continue;
                }
            }
        }

        i += 1;
    }

    diags
}

/// Try to match `keyword` (case-insensitive) at `start` with word boundaries.
/// Returns `Some(end_offset)` on success (end_offset = start + keyword.len()),
/// `None` otherwise.
fn match_keyword(bytes: &[u8], len: usize, skip: &[bool], start: usize, keyword: &[u8]) -> Option<usize> {
    let kw_len = keyword.len();
    if start + kw_len > len {
        return None;
    }

    // Word boundary before.
    let before_ok = start == 0 || !is_word_char(bytes[start - 1]);
    if !before_ok {
        return None;
    }

    // Case-insensitive match.
    let matches = bytes[start..start + kw_len]
        .iter()
        .zip(keyword.iter())
        .all(|(a, b)| a.eq_ignore_ascii_case(b));

    if !matches {
        return None;
    }

    // Ensure none of those bytes are skipped.
    if (start..start + kw_len).any(|k| skip[k]) {
        return None;
    }

    // Word boundary after.
    let after = start + kw_len;
    if after < len && is_word_char(bytes[after]) {
        return None;
    }

    Some(after)
}

/// Scan backward from `before_pos` (skipping whitespace) to check whether the
/// preceding word is ROWS or RANGE.
fn find_rows_or_range_before(bytes: &[u8], skip: &[bool], before_pos: usize) -> bool {
    if before_pos == 0 {
        return false;
    }

    // Skip whitespace backward.
    let mut j = before_pos - 1;
    while j > 0 && is_whitespace(bytes[j]) {
        j -= 1;
    }

    // j is now at the last char of the preceding token.
    // Check if [j-3..=j] == "ROWS" or [j-4..=j] == "RANGE".
    if !is_word_char(bytes[j]) {
        return false;
    }

    // Find start of the word.
    let word_end = j + 1; // exclusive
    let mut word_start = j;
    while word_start > 0 && is_word_char(bytes[word_start - 1]) {
        word_start -= 1;
    }

    let word = &bytes[word_start..word_end];
    // Not skipped.
    if (word_start..word_end).any(|k| skip[k]) {
        return false;
    }

    // Word boundary before the word.
    let before_word_ok = word_start == 0 || !is_word_char(bytes[word_start - 1]);
    if !before_word_ok {
        return false;
    }

    let _ = word_end; // used implicitly via word_end above

    let is_rows = word.eq_ignore_ascii_case(b"ROWS");
    let is_range = word.eq_ignore_ascii_case(b"RANGE");
    is_rows || is_range
}

/// Starting at `pos` (right after BETWEEN + potential whitespace), try to match:
/// `UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING`
/// Returns `Some(end_offset)` on success, `None` otherwise.
fn match_unbounded_preceding_and_following(
    bytes: &[u8],
    len: usize,
    skip: &[bool],
    pos: usize,
) -> Option<usize> {
    let pos = skip_whitespace(bytes, len, pos);

    // UNBOUNDED
    let pos = match_keyword(bytes, len, skip, pos, b"UNBOUNDED")?;
    let pos = skip_whitespace(bytes, len, pos);

    // PRECEDING
    let pos = match_keyword(bytes, len, skip, pos, b"PRECEDING")?;
    let pos = skip_whitespace(bytes, len, pos);

    // AND
    let pos = match_keyword(bytes, len, skip, pos, b"AND")?;
    let pos = skip_whitespace(bytes, len, pos);

    // UNBOUNDED
    let pos = match_keyword(bytes, len, skip, pos, b"UNBOUNDED")?;
    let pos = skip_whitespace(bytes, len, pos);

    // FOLLOWING
    let pos = match_keyword(bytes, len, skip, pos, b"FOLLOWING")?;

    Some(pos)
}

/// Advance past whitespace characters.
fn skip_whitespace(bytes: &[u8], len: usize, mut pos: usize) -> usize {
    while pos < len && is_whitespace(bytes[pos]) {
        pos += 1;
    }
    pos
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

#[inline]
fn is_whitespace(ch: u8) -> bool {
    ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r'
}

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Build a boolean skip-set: `skip[i] == true` means byte `i` is inside a
/// single-quoted string, double-quoted identifier, block comment, or line
/// comment.
fn build_skip_set(bytes: &[u8], len: usize) -> Vec<bool> {
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Single-quoted string: '...' with '' escape.
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..." with "" escape.
        if bytes[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len {
                skip[i] = true;
                if bytes[i] == b'"' {
                    if i + 1 < len && bytes[i + 1] == b'"' {
                        i += 1;
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    i += 1;
                    break;
                }
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
                skip[i] = true;
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[i + 1] = true;
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // Line comment: -- to end of line.
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

        i += 1;
    }

    skip
}
