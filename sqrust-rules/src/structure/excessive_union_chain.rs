use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::{is_word_char, SkipMap};

/// Threshold: flag when the total number of set operators (UNION, UNION ALL,
/// INTERSECT, EXCEPT) in a file reaches or exceeds this value.
const THRESHOLD: usize = 5;

pub struct ExcessiveUnionChain;

impl Rule for ExcessiveUnionChain {
    fn name(&self) -> &'static str {
        "Structure/ExcessiveUnionChain"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let (count, first_pos) = count_set_operators(&ctx.source);

        if count >= THRESHOLD {
            let (line, col) = match first_pos {
                Some(offset) => offset_to_line_col(&ctx.source, offset),
                None => (1, 1),
            };

            vec![Diagnostic {
                rule: self.name(),
                message: format!(
                    "Query contains {count} set operations; consider refactoring into a CTE or derived table for maintainability"
                ),
                line,
                col,
            }]
        } else {
            Vec::new()
        }
    }
}

// ── source-level scan ─────────────────────────────────────────────────────────

/// Count the total number of set operator keywords (UNION, INTERSECT, EXCEPT)
/// in `source` that are code (not inside strings/comments).
///
/// Returns `(count, first_byte_offset)`. `first_byte_offset` is `None` when
/// there are no set operators at all.
///
/// Note: `UNION ALL` is counted as one set operator because the `UNION` keyword
/// already represents the operation; `ALL` is just a modifier.
fn count_set_operators(source: &str) -> (usize, Option<usize>) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);

    // Keywords to count — ordered from longest to shortest so that `UNION ALL`
    // is not double-counted (we consume the full token each time).
    let keywords: &[&[u8]] = &[b"INTERSECT", b"EXCEPT", b"UNION"];

    let mut count = 0usize;
    let mut first_offset: Option<usize> = None;
    let mut i = 0;

    'outer: while i < len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }

        // Word boundary before.
        if i > 0 && is_word_char(bytes[i - 1]) {
            i += 1;
            continue;
        }

        // Try to match any of the set operator keywords.
        for kw in keywords {
            let kw_len = kw.len();
            if i + kw_len > len {
                continue;
            }

            // Case-insensitive match.
            if !bytes[i..i + kw_len].eq_ignore_ascii_case(kw) {
                continue;
            }

            // Word boundary after.
            let after = i + kw_len;
            if after < len && is_word_char(bytes[after]) {
                continue;
            }

            // All bytes must be code.
            if !(i..i + kw_len).all(|k| skip_map.is_code(k)) {
                continue;
            }

            // Matched a set operator keyword.
            count += 1;
            if first_offset.is_none() {
                first_offset = Some(i);
            }

            // Advance past the keyword.
            i += kw_len;
            continue 'outer;
        }

        i += 1;
    }

    (count, first_offset)
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
