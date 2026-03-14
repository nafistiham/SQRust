use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct WhereOnNewLine;

impl Rule for WhereOnNewLine {
    fn name(&self) -> &'static str {
        "Layout/WhereOnNewLine"
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

        if !line_contains_keyword(&line_upper, line_offset, b"FROM", 4, &skip) {
            continue;
        }

        if !line_contains_keyword(&line_upper, line_offset, b"WHERE", 5, &skip) {
            continue;
        }

        let col = line_upper.find("FROM").unwrap_or(0) + 1;
        diags.push(Diagnostic {
            rule: rule_name,
            message: "WHERE clause is on the same line as FROM; prefer placing WHERE on a new line for readability".to_string(),
            line: line_num,
            col,
        });
    }

    diags
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
