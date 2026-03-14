use sqrust_core::{Diagnostic, FileContext, Rule};

/// Flag SELECT statements that contain 3 or more scalar subqueries in the
/// SELECT list. Each scalar subquery executes once per row, so many of them
/// is a performance concern.
pub struct ScalarSubqueryInSelect;

impl Rule for ScalarSubqueryInSelect {
    fn name(&self) -> &'static str {
        "Structure/ScalarSubqueryInSelect"
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

    // Find each top-level SELECT keyword (not inside skip regions).
    // For each SELECT, extract the SELECT list portion (everything up to FROM
    // or end of statement) and count `(SELECT` occurrences within it.
    let mut i = 0;
    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Match SELECT keyword at word boundary.
        let kw = b"SELECT";
        let kw_len = kw.len();
        if i + kw_len > len {
            i += 1;
            continue;
        }

        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(kw.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if !matches {
            i += 1;
            continue;
        }

        let after = i + kw_len;
        let after_ok = after >= len || !is_word_char(bytes[after]);
        if !after_ok {
            i += 1;
            continue;
        }

        // We found a SELECT at position i. Record the SELECT keyword position
        // for diagnostic reporting.
        let select_pos = i;

        // Advance past the SELECT keyword.
        let list_start = i + kw_len;

        // Find the end of the SELECT list: the FROM keyword at the same paren
        // depth, or end of statement (semicolon / end of source).
        // We track paren depth to avoid matching FROM inside subexpressions.
        let list_end = find_select_list_end(bytes, len, &skip, list_start);

        // Count `(SELECT` occurrences within [list_start, list_end).
        let count = count_scalar_subqueries(bytes, len, &skip, list_start, list_end);

        if count >= 3 {
            let (line, col) = offset_to_line_col(source, select_pos);
            diags.push(Diagnostic {
                rule: rule_name,
                message: format!(
                    "SELECT list contains {count} scalar subqueries; each executes once per row and may cause performance issues"
                ),
                line,
                col,
            });
        }

        // Advance past the SELECT keyword to continue scanning.
        i = list_start;
    }

    diags
}

/// Find the byte offset of the end of the SELECT list (exclusive).
/// Scans forward from `start` until it finds a FROM keyword at paren depth 0,
/// or a semicolon, or end of source.
fn find_select_list_end(bytes: &[u8], len: usize, skip: &[bool], start: usize) -> usize {
    let mut i = start;
    let mut depth: usize = 0;

    while i < len {
        if skip[i] {
            i += 1;
            continue;
        }

        let ch = bytes[i];

        if ch == b'(' {
            depth += 1;
            i += 1;
            continue;
        }

        if ch == b')' {
            if depth > 0 {
                depth -= 1;
            }
            i += 1;
            continue;
        }

        if ch == b';' && depth == 0 {
            return i;
        }

        // Check for FROM keyword at depth 0.
        if depth == 0 {
            let kw = b"FROM";
            let kw_len = kw.len();
            if i + kw_len <= len {
                let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
                let matches = bytes[i..i + kw_len]
                    .iter()
                    .zip(kw.iter())
                    .all(|(a, b)| a.eq_ignore_ascii_case(b));
                let after = i + kw_len;
                let after_ok = after >= len || !is_word_char(bytes[after]);

                if before_ok && matches && after_ok {
                    return i;
                }
            }
        }

        i += 1;
    }

    len
}

/// Count occurrences of `(SELECT` (with optional whitespace after `(`) in the
/// source slice `[start, end)`, skipping positions in the skip set.
fn count_scalar_subqueries(
    bytes: &[u8],
    _len: usize,
    skip: &[bool],
    start: usize,
    end: usize,
) -> usize {
    let mut count = 0;
    let mut i = start;

    while i < end {
        if skip[i] {
            i += 1;
            continue;
        }

        if bytes[i] == b'(' {
            // Scan forward past optional whitespace.
            let mut j = i + 1;
            while j < end
                && (bytes[j] == b' '
                    || bytes[j] == b'\t'
                    || bytes[j] == b'\n'
                    || bytes[j] == b'\r')
            {
                j += 1;
            }

            // Check for SELECT keyword.
            let kw = b"SELECT";
            let kw_len = kw.len();
            if j + kw_len <= end {
                let matches = bytes[j..j + kw_len]
                    .iter()
                    .zip(kw.iter())
                    .all(|(a, b)| a.eq_ignore_ascii_case(b));
                let after = j + kw_len;
                let after_ok = after >= end || !is_word_char(bytes[after]);

                if matches && after_ok {
                    // Verify none of the keyword bytes are in skip.
                    let all_code = (j..j + kw_len).all(|k| !skip[k]);
                    if all_code {
                        count += 1;
                    }
                }
            }
        }

        i += 1;
    }

    count
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
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
