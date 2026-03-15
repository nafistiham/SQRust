use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct GroupByOnNewLine;

impl Rule for GroupByOnNewLine {
    fn name(&self) -> &'static str {
        "Layout/GroupByOnNewLine"
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

        // Find position of GROUP BY on this line (if any, outside strings/comments)
        let group_by_pos = find_group_by_pos(&line_upper, line_offset, &skip);
        let group_by_col = match group_by_pos {
            Some(col) => col,
            None => continue,
        };

        // Check if there is non-whitespace content before GROUP BY on this line
        let before = &line[..group_by_col];
        if before.bytes().any(|b| b != b' ' && b != b'\t') {
            diags.push(Diagnostic {
                rule: rule_name,
                message: "GROUP BY clause is not at the start of a new line; prefer placing GROUP BY on a new line for readability".to_string(),
                line: line_num,
                col: group_by_col + 1,
            });
        }
    }

    diags
}

/// Returns the byte offset within the line where `GROUP BY` starts, or `None`.
/// The returned offset is relative to the start of the line.
fn find_group_by_pos(line_upper: &str, line_offset: usize, skip: &[bool]) -> Option<usize> {
    let bytes = line_upper.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i + 5 <= len {
        if &bytes[i..i + 5] == b"GROUP" {
            let abs_group = line_offset + i;
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + 5 >= len || !is_word_char(bytes[i + 5]);
            if before_ok && after_ok && (abs_group >= skip.len() || !skip[abs_group]) {
                // Scan forward for BY (skipping whitespace)
                let mut j = i + 5;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if j + 2 <= len && &bytes[j..j + 2] == b"BY" {
                    let abs_by = line_offset + j;
                    let by_after_ok = j + 2 >= len || !is_word_char(bytes[j + 2]);
                    if by_after_ok && (abs_by >= skip.len() || !skip[abs_by]) {
                        return Some(i);
                    }
                }
            }
        }
        i += 1;
    }

    None
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
