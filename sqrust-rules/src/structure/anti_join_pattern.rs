use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct AntiJoinPattern;

impl Rule for AntiJoinPattern {
    fn name(&self) -> &'static str {
        "Structure/AntiJoinPattern"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();

        // Scan for `NOT` keyword (word-boundary, in code), then check that it
        // is followed by optional whitespace + `IN` keyword, then optional
        // whitespace + `(` + optional whitespace + `SELECT` keyword.
        let mut i = 0;
        while i < len {
            if !skip_map.is_code(i) {
                i += 1;
                continue;
            }

            // Try to match `NOT` at position i.
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            if !before_ok || i + 3 > len {
                i += 1;
                continue;
            }

            let is_not = bytes[i..i + 3].eq_ignore_ascii_case(b"NOT");
            if !is_not {
                i += 1;
                continue;
            }

            let after_not = i + 3;
            // Word boundary after NOT
            let not_boundary = after_not >= len || !is_word_char(bytes[after_not]);
            if !not_boundary {
                i += 1;
                continue;
            }

            // Check all bytes of NOT are in code
            let not_all_code = (i..i + 3).all(|k| skip_map.is_code(k));
            if !not_all_code {
                i += 1;
                continue;
            }

            // Skip whitespace after NOT
            let mut j = after_not;
            while j < len && matches!(bytes[j], b' ' | b'\t' | b'\n' | b'\r') {
                j += 1;
            }

            // Match `IN` at j
            if j + 2 > len {
                i += 1;
                continue;
            }

            let is_in = bytes[j..j + 2].eq_ignore_ascii_case(b"IN");
            let in_boundary_after = j + 2 >= len || !is_word_char(bytes[j + 2]);
            let in_all_code = (j..j + 2).all(|k| skip_map.is_code(k));

            if !is_in || !in_boundary_after || !in_all_code {
                i += 1;
                continue;
            }

            // Skip whitespace after IN
            let mut k = j + 2;
            while k < len && matches!(bytes[k], b' ' | b'\t' | b'\n' | b'\r') {
                k += 1;
            }

            // Match `(` at k
            if k >= len || bytes[k] != b'(' || !skip_map.is_code(k) {
                i += 1;
                continue;
            }

            // Skip whitespace after `(`
            let mut m = k + 1;
            while m < len && matches!(bytes[m], b' ' | b'\t' | b'\n' | b'\r') {
                m += 1;
            }

            // Match `SELECT` at m
            if m + 6 > len {
                i += 1;
                continue;
            }

            let is_select = bytes[m..m + 6].eq_ignore_ascii_case(b"SELECT");
            let select_boundary = m + 6 >= len || !is_word_char(bytes[m + 6]);
            let select_all_code = (m..m + 6).all(|k| skip_map.is_code(k));

            if is_select && select_boundary && select_all_code {
                let (line, col) = offset_to_line_col(source, i);
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "NOT IN (SELECT ...) is NULL-unsafe and may perform poorly; prefer NOT EXISTS or a LEFT JOIN ... WHERE ... IS NULL".to_string(),
                    line,
                    col,
                });
            }

            i += 1;
        }

        diags
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
