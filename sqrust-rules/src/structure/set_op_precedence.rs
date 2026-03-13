use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct SetOpPrecedence;

impl Rule for SetOpPrecedence {
    fn name(&self) -> &'static str {
        "Structure/SetOpPrecedence"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        let diags = check_source(self.name(), source);
        diags
    }
}

/// Scan source for mixed set operators (UNION/EXCEPT + INTERSECT) at each
/// syntactic level. A new "level" begins each time we enter a parenthesised
/// group (depth increase). We flag only when the mix occurs within the same
/// parenthesis depth.
fn check_source(rule: &'static str, source: &str) -> Vec<Diagnostic> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);

    // We collect (depth, op_kind, byte_offset) for every set-op keyword we find.
    // op_kind: 0 = UNION/EXCEPT, 1 = INTERSECT
    let mut ops: Vec<(i32, u8, usize)> = Vec::new();
    let mut paren_depth: i32 = 0;

    let mut i = 0;
    while i < len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }

        let b = bytes[i];

        // Track parenthesis depth.
        if b == b'(' {
            paren_depth += 1;
            i += 1;
            continue;
        }
        if b == b')' {
            if paren_depth > 0 {
                paren_depth -= 1;
            }
            i += 1;
            continue;
        }

        // Look for a word-boundary start.
        if !is_word_char(b) || (i > 0 && is_word_char(bytes[i - 1])) {
            i += 1;
            continue;
        }

        // Read the full word.
        let word_start = i;
        let mut j = i;
        while j < len && is_word_char(bytes[j]) {
            j += 1;
        }
        let word_end = j;

        // Confirm entire word is in code region.
        let all_code = (word_start..word_end).all(|k| skip_map.is_code(k));
        if all_code {
            let word = &bytes[word_start..word_end];

            if matches_keyword(word, b"UNION") || matches_keyword(word, b"EXCEPT") {
                ops.push((paren_depth, 0, word_start));
            } else if matches_keyword(word, b"INTERSECT") {
                ops.push((paren_depth, 1, word_start));
            }
        }

        i = word_end;
    }

    // For each distinct paren depth level, check whether both UNION/EXCEPT (kind=0)
    // and INTERSECT (kind=1) appear.
    let mut diags = Vec::new();

    // Collect all distinct depths.
    let mut depths: Vec<i32> = ops.iter().map(|(d, _, _)| *d).collect();
    depths.sort_unstable();
    depths.dedup();

    for depth in depths {
        let ops_at_depth: Vec<(u8, usize)> = ops
            .iter()
            .filter(|(d, _, _)| *d == depth)
            .map(|(_, kind, off)| (*kind, *off))
            .collect();

        let has_union_or_except = ops_at_depth.iter().any(|(k, _)| *k == 0);
        let has_intersect = ops_at_depth.iter().any(|(k, _)| *k == 1);

        if has_union_or_except && has_intersect {
            // Emit one diagnostic at the position of the first UNION/EXCEPT or
            // INTERSECT at this level (whichever comes first in source order).
            let first_offset = ops_at_depth.iter().map(|(_, off)| *off).min().unwrap_or(0);
            let (line, col) = line_col(source, first_offset);
            diags.push(Diagnostic {
                rule,
                message: "Mixing UNION/EXCEPT with INTERSECT without explicit parentheses is ambiguous — INTERSECT has higher precedence than UNION".to_string(),
                line,
                col,
            });
        }
    }

    diags
}

/// Case-insensitive keyword match (word must equal keyword in length and chars).
fn matches_keyword(word: &[u8], keyword: &[u8]) -> bool {
    word.len() == keyword.len()
        && keyword
            .iter()
            .zip(word.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
