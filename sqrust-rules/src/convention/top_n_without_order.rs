use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct TopNWithoutOrder;

impl Rule for TopNWithoutOrder {
    fn name(&self) -> &'static str {
        "Convention/TopNWithoutOrder"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        find_violations(&ctx.source, self.name())
    }
}

fn build_skip_set(source: &str) -> std::collections::HashSet<usize> {
    let mut skip = std::collections::HashSet::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    while i < len {
        if bytes[i] == b'\'' {
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        skip.insert(i);
                        i += 2;
                    } else {
                        i += 1;
                        break;
                    }
                } else {
                    skip.insert(i);
                    i += 1;
                }
            }
        } else if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            while i < len && bytes[i] != b'\n' {
                skip.insert(i);
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    skip
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Find position of `keyword` (case-insensitive, word-bounded) starting from `start`,
/// skipping bytes in `skip`. Returns None if not found before `limit`.
fn find_keyword(
    bytes: &[u8],
    skip: &std::collections::HashSet<usize>,
    start: usize,
    limit: usize,
    keyword: &[u8],
) -> Option<usize> {
    let kw_len = keyword.len();
    let mut i = start;
    while i + kw_len <= limit {
        if skip.contains(&i) {
            i += 1;
            continue;
        }
        // Word boundary before
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if before_ok && bytes[i..i + kw_len].eq_ignore_ascii_case(keyword) {
            // Check no bytes in keyword are in skip (inside string/comment)
            let all_code = (0..kw_len).all(|k| !skip.contains(&(i + k)));
            if all_code {
                // Word boundary after
                let after = i + kw_len;
                let after_ok = after >= bytes.len() || !is_word_char(bytes[after]);
                if after_ok {
                    return Some(i);
                }
            }
        }
        i += 1;
    }
    None
}

fn find_violations(source: &str, rule_name: &'static str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();

    if len == 0 {
        return Vec::new();
    }

    let skip = build_skip_set(source);
    let mut diags = Vec::new();

    // Search for SELECT TOP keyword sequence.
    let select_kw = b"SELECT";
    let select_len = select_kw.len();
    let top_kw = b"TOP";

    let mut i = 0;
    while i < len {
        if skip.contains(&i) {
            i += 1;
            continue;
        }

        // Try to match SELECT at position i with word boundaries.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        if i + select_len > len {
            break;
        }

        if !bytes[i..i + select_len].eq_ignore_ascii_case(select_kw) {
            i += 1;
            continue;
        }

        // Ensure all SELECT chars are code
        let all_code = (0..select_len).all(|k| !skip.contains(&(i + k)));
        if !all_code {
            i += 1;
            continue;
        }

        let select_end = i + select_len;
        // Word boundary after SELECT
        if select_end < len && is_word_char(bytes[select_end]) {
            i += 1;
            continue;
        }

        // Advance past SELECT, skip whitespace to find TOP.
        let mut j = select_end;
        while j < len && !skip.contains(&j) && (bytes[j] == b' ' || bytes[j] == b'\t') {
            j += 1;
        }

        // Check if next word-bounded token is TOP
        if j >= len || skip.contains(&j) {
            i += 1;
            continue;
        }

        let top_len = top_kw.len();
        if j + top_len > len {
            i += 1;
            continue;
        }

        if !bytes[j..j + top_len].eq_ignore_ascii_case(top_kw) {
            i += 1;
            continue;
        }

        let all_top_code = (0..top_len).all(|k| !skip.contains(&(j + k)));
        if !all_top_code {
            i += 1;
            continue;
        }

        let top_end = j + top_len;
        let top_after_ok = top_end >= len || !is_word_char(bytes[top_end]);
        if !top_after_ok {
            i += 1;
            continue;
        }

        // Found SELECT TOP. Now determine the end of this statement:
        // scan forward to the next semicolon, or end of string.
        // Within this range, look for ORDER BY (but not past a new SELECT not
        // inside a subquery — for simplicity we stop at semicolons only).
        let stmt_end = {
            let mut end = top_end;
            while end < len {
                if !skip.contains(&end) && bytes[end] == b';' {
                    break;
                }
                end += 1;
            }
            end
        };

        // Look for ORDER BY within stmt_end
        let order_pos = find_keyword(bytes, &skip, top_end, stmt_end, b"ORDER");
        let has_order_by = if let Some(ord_pos) = order_pos {
            // Verify it's ORDER BY (skip whitespace then look for BY)
            let mut k = ord_pos + 5; // len("ORDER")
            while k < stmt_end && !skip.contains(&k) && (bytes[k] == b' ' || bytes[k] == b'\t' || bytes[k] == b'\n' || bytes[k] == b'\r') {
                k += 1;
            }
            if k + 2 <= stmt_end && bytes[k..k + 2].eq_ignore_ascii_case(b"BY") {
                let all_by_code = !skip.contains(&k) && !skip.contains(&(k + 1));
                let by_after = k + 2;
                let by_after_ok = by_after >= stmt_end || !is_word_char(bytes[by_after]);
                all_by_code && by_after_ok
            } else {
                false
            }
        } else {
            false
        };

        if !has_order_by {
            let (line, col) = line_col(source, j);
            diags.push(Diagnostic {
                rule: rule_name,
                message: "SELECT TOP without ORDER BY returns non-deterministic rows — add ORDER BY to get consistent results, and consider using LIMIT/FETCH FIRST for portable top-N queries".to_string(),
                line,
                col,
            });
        }

        i = stmt_end + 1;
    }

    diags
}
