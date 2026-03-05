use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::SkipMap;

pub struct NoSelectAll;

impl Rule for NoSelectAll {
    fn name(&self) -> &'static str {
        "NoSelectAll"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();

        // Scan for SELECT followed by optional whitespace and then ALL (with a
        // word boundary after ALL). Case-insensitive.
        //
        // Note: UNION ALL must NOT be flagged; only SELECT ALL must be flagged.
        let mut i = 0;
        while i < len {
            // Skip non-code bytes.
            if !skip_map.is_code(i) {
                i += 1;
                continue;
            }

            // Try to match SELECT at position i.
            if !matches_keyword_at(bytes, len, &skip_map, i, b"SELECT") {
                i += 1;
                continue;
            }

            // Verify word boundary before SELECT.
            if i > 0 && is_word_char(bytes[i - 1]) {
                i += 1;
                continue;
            }

            let select_start = i;
            let select_end = i + 6; // length of "SELECT"

            // Verify word boundary after SELECT.
            if select_end < len && is_word_char(bytes[select_end]) {
                i += 1;
                continue;
            }

            // After SELECT, skip whitespace.
            let mut j = select_end;
            while j < len && skip_map.is_code(j) && is_whitespace(bytes[j]) {
                j += 1;
            }

            // There must be at least one whitespace character between SELECT and ALL.
            if j == select_end {
                i += 1;
                continue;
            }

            // Try to match ALL at position j.
            if !matches_keyword_at(bytes, len, &skip_map, j, b"ALL") {
                i += 1;
                continue;
            }

            // Verify word boundary before ALL.
            if j > 0 && is_word_char(bytes[j - 1]) {
                i += 1;
                continue;
            }

            let all_end = j + 3; // length of "ALL"

            // Verify word boundary after ALL — this guards against SELECT ALLCOLS.
            let after_all_ok = all_end >= len || !is_word_char(bytes[all_end]);
            if !after_all_ok {
                i += 1;
                continue;
            }

            let (line, col) = line_col(source, select_start);
            diags.push(Diagnostic {
                rule: self.name(),
                message: "SELECT ALL is redundant; ALL is the default behavior, use SELECT without ALL"
                    .to_string(),
                line,
                col,
            });

            // Advance past the full SELECT ALL match.
            i = all_end;
        }

        diags
    }
}

/// Returns true if the bytes at `pos` spell out `keyword` (case-insensitive)
/// and every byte of the match is code (not inside a string/comment).
fn matches_keyword_at(
    bytes: &[u8],
    len: usize,
    skip_map: &SkipMap,
    pos: usize,
    keyword: &[u8],
) -> bool {
    let kw_len = keyword.len();
    if pos + kw_len > len {
        return false;
    }
    (0..kw_len).all(|k| {
        skip_map.is_code(pos + k) && bytes[pos + k].eq_ignore_ascii_case(&keyword[k])
    })
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

#[inline]
fn is_whitespace(ch: u8) -> bool {
    ch == b' ' || ch == b'\t' || ch == b'\n' || ch == b'\r'
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
