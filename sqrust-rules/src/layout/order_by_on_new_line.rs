use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct OrderByOnNewLine;

impl Rule for OrderByOnNewLine {
    fn name(&self) -> &'static str {
        "Layout/OrderByOnNewLine"
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

    for (line_idx, line) in source.split('\n').enumerate() {
        let line_num = line_idx + 1;
        let line_offset = line_offset_in_source(source, line_idx);
        let line_upper = line.to_uppercase();

        // Must contain ORDER BY
        if !line_contains_order_by(&line_upper, line_offset, &skip) {
            continue;
        }

        // Must also contain WHERE, GROUP BY, or HAVING on the same line
        let has_where = line_contains_keyword(&line_upper, line_offset, b"WHERE", 5, &skip);
        let has_group_by = line_contains_group_by(&line_upper, line_offset, &skip);
        let has_having = line_contains_keyword(&line_upper, line_offset, b"HAVING", 6, &skip);

        if !has_where && !has_group_by && !has_having {
            continue;
        }

        let col = find_order_by_col(&line_upper) + 1;
        diags.push(Diagnostic {
            rule: rule_name,
            message: "ORDER BY clause is on the same line as a preceding clause; prefer placing ORDER BY on a new line for readability".to_string(),
            line: line_num,
            col,
        });
    }

    diags
}

/// Returns the 0-based character offset of the ORDER keyword on the line, or 0 if not found.
fn find_order_by_col(line_upper: &str) -> usize {
    let bytes = line_upper.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + 5 <= len {
        if &bytes[i..i + 5] == b"ORDER" {
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + 5 >= len || !is_word_char(bytes[i + 5]);
            if before_ok && after_ok {
                // Scan forward for BY
                let mut j = i + 5;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if j + 2 <= len && &bytes[j..j + 2] == b"BY" {
                    let by_after_ok = j + 2 >= len || !is_word_char(bytes[j + 2]);
                    if by_after_ok {
                        return i;
                    }
                }
            }
        }
        i += 1;
    }

    0
}

/// Returns true if the line contains the `ORDER BY` two-keyword sequence outside strings/comments.
fn line_contains_order_by(line_upper: &str, line_offset: usize, skip: &[bool]) -> bool {
    let bytes = line_upper.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + 5 <= len {
        if &bytes[i..i + 5] == b"ORDER" {
            let abs_order = line_offset + i;
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + 5 >= len || !is_word_char(bytes[i + 5]);
            if before_ok && after_ok && (abs_order >= skip.len() || !skip[abs_order]) {
                // Scan forward for BY
                let mut j = i + 5;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if j + 2 <= len && &bytes[j..j + 2] == b"BY" {
                    let abs_by = line_offset + j;
                    let by_after_ok = j + 2 >= len || !is_word_char(bytes[j + 2]);
                    if by_after_ok && (abs_by >= skip.len() || !skip[abs_by]) {
                        return true;
                    }
                }
            }
        }
        i += 1;
    }

    false
}

/// Returns true if the line contains the `GROUP BY` two-keyword sequence outside strings/comments.
fn line_contains_group_by(line_upper: &str, line_offset: usize, skip: &[bool]) -> bool {
    let bytes = line_upper.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + 5 <= len {
        if &bytes[i..i + 5] == b"GROUP" {
            let abs_group = line_offset + i;
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + 5 >= len || !is_word_char(bytes[i + 5]);
            if before_ok && after_ok && (abs_group >= skip.len() || !skip[abs_group]) {
                let mut j = i + 5;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if j + 2 <= len && &bytes[j..j + 2] == b"BY" {
                    let abs_by = line_offset + j;
                    let by_after_ok = j + 2 >= len || !is_word_char(bytes[j + 2]);
                    if by_after_ok && (abs_by >= skip.len() || !skip[abs_by]) {
                        return true;
                    }
                }
            }
        }
        i += 1;
    }

    false
}

/// Returns true if the line contains the given keyword (upper-cased) outside strings/comments.
fn line_contains_keyword(
    line_upper: &str,
    line_offset: usize,
    keyword: &[u8],
    kw_len: usize,
    skip: &[bool],
) -> bool {
    let bytes = line_upper.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + kw_len <= len {
        if &bytes[i..i + kw_len] == keyword {
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

/// Returns the byte offset where line `line_idx` starts in `source`.
fn line_offset_in_source(source: &str, line_idx: usize) -> usize {
    if line_idx == 0 {
        return 0;
    }
    let mut offset = 0;
    for (i, line) in source.split('\n').enumerate() {
        if i == line_idx {
            return offset;
        }
        offset += line.len() + 1; // +1 for the '\n'
    }
    offset
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
