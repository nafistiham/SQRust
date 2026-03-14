use sqrust_core::{Diagnostic, FileContext, Rule};

/// Flag `CROSS APPLY` and `OUTER APPLY` which are SQL Server / PostgreSQL-specific
/// table-valued function join syntax not supported in standard SQL or most
/// analytical databases.
pub struct CrossApply;

impl Rule for CrossApply {
    fn name(&self) -> &'static str {
        "Structure/CrossApply"
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

    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Word boundary before current position.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Try to match CROSS APPLY.
        if let Some(end) = match_two_word_keyword(bytes, len, &skip, i, b"CROSS", b"APPLY") {
            let (line, col) = offset_to_line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: "CROSS APPLY is SQL Server/PostgreSQL-specific; use a LATERAL JOIN for standard SQL".to_string(),
                line,
                col,
            });
            i = end;
            continue;
        }

        // Try to match OUTER APPLY.
        if let Some(end) = match_two_word_keyword(bytes, len, &skip, i, b"OUTER", b"APPLY") {
            let (line, col) = offset_to_line_col(source, i);
            diags.push(Diagnostic {
                rule: rule_name,
                message: "OUTER APPLY is SQL Server/PostgreSQL-specific; use a LEFT JOIN LATERAL for standard SQL".to_string(),
                line,
                col,
            });
            i = end;
            continue;
        }

        i += 1;
    }

    diags
}

/// Try to match two keywords (word1 followed by optional whitespace then word2)
/// starting at `start`. Returns `Some(end_offset)` if matched, `None` otherwise.
/// `end_offset` is one past the last character of the second keyword.
fn match_two_word_keyword(
    bytes: &[u8],
    len: usize,
    skip: &[bool],
    start: usize,
    word1: &[u8],
    word2: &[u8],
) -> Option<usize> {
    let w1_len = word1.len();
    let w2_len = word2.len();

    if start + w1_len > len {
        return None;
    }

    // Match word1 (case-insensitive).
    let matches_w1 = bytes[start..start + w1_len]
        .iter()
        .zip(word1.iter())
        .all(|(a, b)| a.eq_ignore_ascii_case(b));

    if !matches_w1 {
        return None;
    }

    // Ensure none of word1's bytes are skipped.
    if (start..start + w1_len).any(|k| skip[k]) {
        return None;
    }

    // After word1 must be a word boundary (not a word char).
    let after_w1 = start + w1_len;
    if after_w1 < len && is_word_char(bytes[after_w1]) {
        return None;
    }

    // Skip whitespace between word1 and word2.
    let mut j = after_w1;
    while j < len && is_whitespace(bytes[j]) {
        j += 1;
    }

    if j + w2_len > len {
        return None;
    }

    // Match word2 (case-insensitive).
    let matches_w2 = bytes[j..j + w2_len]
        .iter()
        .zip(word2.iter())
        .all(|(a, b)| a.eq_ignore_ascii_case(b));

    if !matches_w2 {
        return None;
    }

    // Ensure none of word2's bytes are skipped.
    if (j..j + w2_len).any(|k| skip[k]) {
        return None;
    }

    // After word2 must be a word boundary.
    let after_w2 = j + w2_len;
    if after_w2 < len && is_word_char(bytes[after_w2]) {
        return None;
    }

    Some(after_w2)
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
