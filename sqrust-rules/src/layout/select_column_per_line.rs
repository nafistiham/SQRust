use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct SelectColumnPerLine;

impl Rule for SelectColumnPerLine {
    fn name(&self) -> &'static str {
        "Layout/SelectColumnPerLine"
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
    let lines: Vec<&str> = source.split('\n').collect();
    let line_count = lines.len();
    let mut diags = Vec::new();

    let mut i = 0;

    while i < line_count {
        // Look for a line containing the SELECT keyword (outside strings/comments).
        let line_offset = offset_of_line(&lines, i);
        let line_upper = lines[i].to_uppercase();

        if !contains_keyword_on_line(&line_upper, "SELECT", lines[i], line_offset, &skip) {
            i += 1;
            continue;
        }

        // Found a SELECT. Now scan forward for lines in the SELECT list.
        // The SELECT list ends when we hit FROM, UNION, INTERSECT, EXCEPT,
        // ORDER BY, GROUP BY, HAVING, LIMIT, WHERE, or end of source.
        // Lines strictly between the SELECT line and the terminator are candidates.
        let _select_line = i;
        i += 1;

        while i < line_count {
            let candidate_offset = offset_of_line(&lines, i);
            let candidate_upper = lines[i].to_uppercase();

            // Check if this line ends the SELECT list.
            if line_starts_clause(&candidate_upper, lines[i], candidate_offset, &skip) {
                break;
            }

            // This line is inside the SELECT list. Check if it has a comma
            // that is NOT the last non-whitespace/comment character on the line —
            // which would indicate multiple columns on the same line.
            if has_inline_comma(lines[i], candidate_offset, &skip) {
                // Find the column of the comma.
                let col = find_inline_comma_col(lines[i], candidate_offset, &skip);
                diags.push(Diagnostic {
                    rule: rule_name,
                    message: "Multiple SELECT columns on one line; prefer placing each column expression on its own line".to_string(),
                    line: i + 1,
                    col,
                });
            }

            i += 1;
        }

        // Do NOT increment i here; the terminator line (FROM etc.) may itself
        // contain a SELECT (e.g. subquery), but we let the outer loop handle it.
        // We just continue from the current i (which is at the terminator line).
    }

    diags
}

/// Returns the byte offset of the start of `lines[idx]` within the original source.
fn offset_of_line(lines: &[&str], idx: usize) -> usize {
    let mut offset = 0;
    for (i, line) in lines.iter().enumerate() {
        if i == idx {
            return offset;
        }
        offset += line.len() + 1; // +1 for '\n'
    }
    offset
}

/// Returns true if the line contains the given keyword (word-bounded, case-insensitive)
/// outside of strings/comments.
fn contains_keyword_on_line(
    line_upper: &str,
    keyword: &str,
    _line: &str,
    line_offset: usize,
    skip: &[bool],
) -> bool {
    let kw = keyword.as_bytes();
    let kw_len = kw.len();
    let bytes = line_upper.as_bytes();
    let len = bytes.len();

    let mut i = 0;
    while i + kw_len <= len {
        if bytes[i..i + kw_len].eq_ignore_ascii_case(kw) {
            let abs = line_offset + i;
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + kw_len >= len || !is_word_char(bytes[i + kw_len]);
            if before_ok && after_ok && (abs >= skip.len() || !skip[abs]) {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Returns true if this line begins a SQL clause that terminates a SELECT list.
fn line_starts_clause(
    line_upper: &str,
    line: &str,
    line_offset: usize,
    skip: &[bool],
) -> bool {
    const TERMINATORS: &[&str] = &[
        "FROM", "WHERE", "GROUP", "HAVING", "ORDER", "LIMIT",
        "UNION", "INTERSECT", "EXCEPT", "WINDOW",
    ];
    let trimmed = line_upper.trim_start();
    let leading_whitespace = line_upper.len() - trimmed.len();
    let trimmed_offset = line_offset + leading_whitespace;

    for kw in TERMINATORS {
        let kw_bytes = kw.as_bytes();
        let kw_len = kw_bytes.len();
        let tb = trimmed.as_bytes();
        if tb.len() >= kw_len && tb[..kw_len].eq_ignore_ascii_case(kw_bytes) {
            let after_ok = tb.len() == kw_len || !is_word_char(tb[kw_len]);
            if after_ok && (trimmed_offset >= skip.len() || !skip[trimmed_offset]) {
                return true;
            }
        }
    }

    // Also stop if the line itself contains a SELECT (subquery) — handled by outer loop
    let _ = line;
    false
}

/// Returns true if the line has a comma that is NOT at the trailing position
/// (i.e. not the last non-whitespace token before a comment), indicating that
/// two or more column expressions share this line.
///
/// A trailing comma at end-of-line (e.g. `  a,`) does NOT indicate multiple
/// columns on the same line — it just separates this column from the next one.
/// Only a comma with non-whitespace code AFTER it counts as "inline".
fn has_inline_comma(line: &str, line_offset: usize, skip: &[bool]) -> bool {
    find_inline_comma_col(line, line_offset, skip) > 0
}

/// Returns the 1-indexed column of the first inline comma, or 0 if none.
fn find_inline_comma_col(line: &str, line_offset: usize, skip: &[bool]) -> usize {
    let bytes = line.as_bytes();
    let len = bytes.len();

    let mut i = 0;
    while i < len {
        let abs = line_offset + i;
        if abs < skip.len() && skip[abs] {
            i += 1;
            continue;
        }

        if bytes[i] == b',' {
            // Check if there is any non-whitespace code character after this comma
            // on the same line (outside skip).
            let mut j = i + 1;
            while j < len {
                let abs_j = line_offset + j;
                if abs_j < skip.len() && skip[abs_j] {
                    j += 1;
                    continue;
                }
                if bytes[j] != b' ' && bytes[j] != b'\t' {
                    // There is a non-whitespace, non-comment character after the comma.
                    return i + 1; // 1-indexed column of the comma
                }
                j += 1;
            }
            // No non-whitespace code after this comma — it's a trailing comma.
        }

        i += 1;
    }

    0
}

#[inline]
fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Build a boolean skip-set: `skip[i] == true` means byte `i` is inside a
/// single-quoted string, double-quoted identifier, block comment, or line comment.
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
