use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct BlankLineAfterCte;

impl Rule for BlankLineAfterCte {
    fn name(&self) -> &'static str {
        "Layout/BlankLineAfterCte"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            if !skip.is_code(i) {
                i += 1;
                continue;
            }

            // Look for WITH keyword at word boundary
            if !is_word_char(bytes[i]) || (i > 0 && is_word_char(bytes[i - 1])) {
                i += 1;
                continue;
            }
            let ws = i;
            let mut we = i;
            while we < len && is_word_char(bytes[we]) { we += 1; }
            let word = &bytes[ws..we];

            if !word.eq_ignore_ascii_case(b"WITH") {
                i = we;
                continue;
            }

            // Found WITH — now scan CTE definitions
            i = we;
            scan_ctes(bytes, len, &skip, &mut i, source, self.name(), &mut diags);
        }

        diags
    }
}

fn scan_ctes(
    bytes: &[u8],
    len: usize,
    skip: &SkipMap,
    pos: &mut usize,
    source: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    loop {
        // Skip to opening `(` of the CTE body (past CTE name and AS keyword)
        skip_to_open_paren(bytes, len, skip, pos);
        if *pos >= len { break; }
        if bytes[*pos] != b'(' { break; }

        // Track paren depth to find the matching closing `)`
        let mut depth = 0usize;
        while *pos < len {
            if skip.is_code(*pos) {
                if bytes[*pos] == b'(' { depth += 1; }
                else if bytes[*pos] == b')' {
                    if depth > 0 { depth -= 1; }
                    if depth == 0 {
                        *pos += 1;
                        break;
                    }
                }
            }
            *pos += 1;
        }

        // After the closing `)`.
        // Skip whitespace to find the comma (or end of CTEs).
        while *pos < len && is_space(bytes[*pos]) {
            *pos += 1;
        }

        // If next non-whitespace is not `,` then no more CTEs.
        if *pos >= len || bytes[*pos] != b',' {
            break;
        }

        let comma_pos = *pos;
        *pos += 1; // skip comma

        // Collect gap from comma to next opening `(` — this is where the blank line should be
        let gap_start = *pos;
        // Find the next `(` in code context
        let mut j = *pos;
        while j < len {
            if skip.is_code(j) && bytes[j] == b'(' { break; }
            j += 1;
        }
        let gap = &bytes[gap_start..j.min(len)];

        // Check if the gap contains a blank line (two or more consecutive newlines)
        if !has_blank_line(gap) {
            let (line, col) = offset_to_line_col(source, comma_pos);
            diags.push(Diagnostic {
                rule,
                message: "Consecutive CTE definitions should be separated by a blank line".to_string(),
                line,
                col,
            });
        }

        // pos is already past the comma; loop will call skip_to_open_paren from there
        // which will find the next `(`
    }
}

fn skip_to_open_paren(bytes: &[u8], len: usize, skip: &SkipMap, pos: &mut usize) {
    while *pos < len {
        if skip.is_code(*pos) && bytes[*pos] == b'(' {
            return;
        }
        *pos += 1;
    }
}

fn is_space(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
}

fn has_blank_line(bytes: &[u8]) -> bool {
    let mut newline_count = 0u32;
    for &b in bytes {
        if b == b'\n' {
            newline_count += 1;
            if newline_count >= 2 { return true; }
        } else if b != b'\r' {
            // Non-newline, non-CR resets the blank-line count
            newline_count = 0;
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
