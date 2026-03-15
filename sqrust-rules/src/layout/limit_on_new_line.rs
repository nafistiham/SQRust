use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct LimitOnNewLine;

impl Rule for LimitOnNewLine {
    fn name(&self) -> &'static str {
        "Layout/LimitOnNewLine"
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

        // The line must contain ORDER BY
        if !line_contains_order_by(&line_upper, line_offset, &skip) {
            continue;
        }

        // Check for LIMIT on same line as ORDER BY
        if line_contains_keyword(&line_upper, line_offset, b"LIMIT", 5, &skip) {
            let col = line_upper.find("LIMIT").unwrap_or(0) + 1;
            diags.push(Diagnostic {
                rule: rule_name,
                message: "LIMIT clause is on the same line as ORDER BY; prefer placing LIMIT on a new line for readability".to_string(),
                line: line_num,
                col,
            });
            continue;
        }

        // Check for FETCH FIRST/NEXT on same line as ORDER BY
        if let Some(col) = find_fetch_clause_col(&line_upper, line_offset, &skip) {
            diags.push(Diagnostic {
                rule: rule_name,
                message: "FETCH FIRST/NEXT clause is on the same line as ORDER BY; prefer placing FETCH on a new line for readability".to_string(),
                line: line_num,
                col,
            });
        }
    }

    diags
}

/// Returns the 1-based column of the FETCH keyword if a `FETCH FIRST`/`FETCH NEXT` sequence
/// is found outside strings/comments, or `None` otherwise.
fn find_fetch_clause_col(line_upper: &str, line_offset: usize, skip: &[bool]) -> Option<usize> {
    let bytes = line_upper.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + 5 <= len {
        if &bytes[i..i + 5] == b"FETCH" {
            let abs_fetch = line_offset + i;
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + 5 >= len || !is_word_char(bytes[i + 5]);
            if before_ok && after_ok && (abs_fetch >= skip.len() || !skip[abs_fetch]) {
                // Scan forward for FIRST or NEXT
                let mut j = i + 5;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                let is_first = j + 5 <= len && &bytes[j..j + 5] == b"FIRST";
                let is_next = j + 4 <= len && &bytes[j..j + 4] == b"NEXT";
                if is_first || is_next {
                    let kw_end = j + if is_first { 5 } else { 4 };
                    let word_after_ok = kw_end >= len || !is_word_char(bytes[kw_end]);
                    if word_after_ok {
                        let abs_kw = line_offset + j;
                        if abs_kw >= skip.len() || !skip[abs_kw] {
                            return Some(i + 1); // 1-based col of FETCH
                        }
                    }
                }
            }
        }
        i += 1;
    }

    None
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
