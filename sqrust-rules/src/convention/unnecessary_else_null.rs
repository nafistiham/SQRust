use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::SkipMap;

pub struct UnnecessaryElseNull;

impl Rule for UnnecessaryElseNull {
    fn name(&self) -> &'static str {
        "UnnecessaryElseNull"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();

        // We scan for ELSE followed by optional whitespace and then NULL (with
        // a word boundary after NULL). Everything is case-insensitive.
        //
        // The pattern is: E L S E  <whitespace>+  N U L L  <non-word-char or EOF>
        //
        // We look for the letter 'E' (start of ELSE) and then verify the rest.
        let mut i = 0;
        while i < len {
            // Skip non-code bytes early.
            if !skip_map.is_code(i) {
                i += 1;
                continue;
            }

            // Try to match ELSE at position i.
            if !matches_keyword_at(bytes, len, &skip_map, i, b"ELSE") {
                i += 1;
                continue;
            }

            // Verify word boundary before ELSE.
            if i > 0 && is_word_char(bytes[i - 1]) {
                i += 1;
                continue;
            }

            let else_start = i;
            let else_end = i + 4; // length of "ELSE"

            // After ELSE, skip whitespace (within code).
            let mut j = else_end;
            while j < len && skip_map.is_code(j) && is_whitespace(bytes[j]) {
                j += 1;
            }

            // There must be at least one whitespace between ELSE and NULL.
            if j == else_end {
                i += 1;
                continue;
            }

            // Now try to match NULL at position j.
            if !matches_keyword_at(bytes, len, &skip_map, j, b"NULL") {
                i += 1;
                continue;
            }

            // Verify word boundary before NULL.
            if j > 0 && is_word_char(bytes[j - 1]) {
                i += 1;
                continue;
            }

            let null_end = j + 4; // length of "NULL"

            // Verify word boundary after NULL (must not be inside a larger identifier).
            let after_null_ok = null_end >= len || !is_word_char(bytes[null_end]);
            if !after_null_ok {
                i += 1;
                continue;
            }

            // All bytes of NULL must be code (not inside a string/comment).
            let null_all_code = (j..null_end).all(|k| skip_map.is_code(k));
            if !null_all_code {
                i += 1;
                continue;
            }

            let (line, col) = line_col(source, else_start);
            diags.push(Diagnostic {
                rule: self.name(),
                message: "ELSE NULL is redundant in CASE expression; omit ELSE to get the same result"
                    .to_string(),
                line,
                col,
            });

            // Advance past the full match to avoid double-counting.
            i = null_end;
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
