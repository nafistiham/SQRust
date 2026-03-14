use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct BlankLineBetweenCTEs;

impl Rule for BlankLineBetweenCTEs {
    fn name(&self) -> &'static str {
        "Layout/BlankLineBetweenCTEs"
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
        // Skip strings and comments.
        if skip[i] {
            i += 1;
            continue;
        }

        // Find standalone WITH keyword.
        if !is_word_char(bytes[i]) || (i > 0 && is_word_char(bytes[i - 1])) {
            i += 1;
            continue;
        }

        // Read the word starting at i.
        let ws = i;
        let mut we = i;
        while we < len && is_word_char(bytes[we]) {
            we += 1;
        }
        let word = &bytes[ws..we];

        if !word.eq_ignore_ascii_case(b"WITH") {
            i = we;
            continue;
        }

        // Found WITH — scan CTE definitions.
        i = we;
        scan_ctes(bytes, len, &skip, &mut i, source, rule_name, &mut diags);
    }

    diags
}

fn scan_ctes(
    bytes: &[u8],
    len: usize,
    skip: &[bool],
    pos: &mut usize,
    source: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    loop {
        // Skip to the opening `(` of the CTE body (past CTE name and AS keyword).
        skip_to_open_paren(bytes, len, skip, pos);
        if *pos >= len {
            break;
        }
        if bytes[*pos] != b'(' {
            break;
        }

        // Track paren depth to find the matching closing `)`.
        let mut depth = 0usize;
        while *pos < len {
            if !skip[*pos] {
                if bytes[*pos] == b'(' {
                    depth += 1;
                } else if bytes[*pos] == b')' {
                    if depth > 0 {
                        depth -= 1;
                    }
                    if depth == 0 {
                        *pos += 1;
                        break;
                    }
                }
            }
            *pos += 1;
        }

        // After the closing `)`, skip whitespace to find comma or end.
        while *pos < len && is_whitespace(bytes[*pos]) {
            *pos += 1;
        }

        // If next non-whitespace char is not `,` then no more CTEs.
        if *pos >= len || bytes[*pos] != b',' {
            break;
        }

        let comma_pos = *pos;
        *pos += 1; // skip comma

        // Scan the gap from after the comma to the next opening `(`.
        // Find where the next CTE body `(` is.
        let gap_start = *pos;
        let mut j = *pos;
        while j < len {
            if !skip[j] && bytes[j] == b'(' {
                break;
            }
            j += 1;
        }
        let gap = &bytes[gap_start..j.min(len)];

        // Check if the gap contains a blank line (at least two newlines with only
        // whitespace between them).
        if !has_blank_line(gap) {
            let (line, col) = offset_to_line_col(source, comma_pos);
            diags.push(Diagnostic {
                rule,
                message: "Missing blank line between CTE definitions — add a blank line after the ',' to improve readability".to_string(),
                line,
                col,
            });
        }

        // pos is past the comma; the next iteration will call skip_to_open_paren
        // which will find the `(` of the next CTE body.
    }
}

fn skip_to_open_paren(bytes: &[u8], len: usize, skip: &[bool], pos: &mut usize) {
    while *pos < len {
        if !skip[*pos] && bytes[*pos] == b'(' {
            return;
        }
        *pos += 1;
    }
}

#[inline]
fn is_whitespace(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Returns true if `bytes` contains at least one blank line.
/// A blank line is a line containing only whitespace (spaces/tabs) between two newlines,
/// OR the sequence \n\n (two consecutive newlines with nothing between them).
fn has_blank_line(bytes: &[u8]) -> bool {
    let mut i = 0;
    let len = bytes.len();
    while i < len {
        if bytes[i] == b'\n' {
            // Look ahead: skip spaces and tabs, then see if we hit another \n.
            let mut j = i + 1;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\r') {
                j += 1;
            }
            if j < len && bytes[j] == b'\n' {
                return true;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    false
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
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
