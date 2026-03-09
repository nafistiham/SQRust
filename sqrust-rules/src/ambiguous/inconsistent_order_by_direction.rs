use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::{is_word_char, SkipMap};
use super::group_by_position::{match_keyword, skip_whitespace};

pub struct InconsistentOrderByDirection;

impl Rule for InconsistentOrderByDirection {
    fn name(&self) -> &'static str {
        "Ambiguous/InconsistentOrderByDirection"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);
        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            if !skip_map.is_code(i) {
                i += 1;
                continue;
            }

            if let Some(after_order) = match_keyword(bytes, &skip_map, i, b"ORDER") {
                let after_ws = skip_whitespace(bytes, after_order);
                if let Some(after_by) = match_keyword(bytes, &skip_map, after_ws, b"BY") {
                    if is_inconsistent_direction(bytes, &skip_map, after_by) {
                        let (line, col) = offset_to_line_col(source, i);
                        diags.push(Diagnostic {
                            rule: "Ambiguous/InconsistentOrderByDirection",
                            message: "ORDER BY mixes explicit direction (ASC/DESC) with implicit; specify direction for all columns".to_string(),
                            line,
                            col,
                        });
                    }
                    i = after_by;
                    continue;
                }
            }

            i += 1;
        }

        diags
    }
}

/// Stop keywords that terminate an ORDER BY item list.
const ORDER_BY_STOP: &[&[u8]] = &[
    b"LIMIT", b"UNION", b"INTERSECT", b"EXCEPT", b"FETCH", b"OFFSET", b"FOR",
];

/// Converts a byte offset to 1-indexed (line, col).
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Returns the last word token in `bytes[start..end]`, scanning backwards,
/// ignoring trailing whitespace.
fn last_word(bytes: &[u8], start: usize, end: usize) -> &[u8] {
    // Trim trailing whitespace.
    let mut e = end;
    while e > start
        && (bytes[e - 1] == b' '
            || bytes[e - 1] == b'\t'
            || bytes[e - 1] == b'\n'
            || bytes[e - 1] == b'\r')
    {
        e -= 1;
    }
    if e <= start {
        return &[];
    }
    // Walk backward over word characters.
    let word_end = e;
    let mut word_start = e;
    while word_start > start && is_word_char(bytes[word_start - 1]) {
        word_start -= 1;
    }
    &bytes[word_start..word_end]
}

/// Scans the ORDER BY item list starting at `start` (right after the BY keyword).
/// Returns true if the clause mixes explicit direction (ASC/DESC) with implicit (no direction).
fn is_inconsistent_direction(bytes: &[u8], skip_map: &SkipMap, start: usize) -> bool {
    let len = bytes.len();
    let mut i = start;
    let mut has_explicit = false;
    let mut has_implicit = false;

    'outer: loop {
        // Skip leading whitespace before item.
        while i < len
            && (bytes[i] == b' '
                || bytes[i] == b'\t'
                || bytes[i] == b'\n'
                || bytes[i] == b'\r')
        {
            i += 1;
        }

        if i >= len {
            break;
        }

        // Semicolon or closing paren terminates.
        if skip_map.is_code(i) && (bytes[i] == b';' || bytes[i] == b')') {
            break;
        }

        // Check stop keywords.
        for &stop in ORDER_BY_STOP {
            if match_keyword(bytes, skip_map, i, stop).is_some() {
                break 'outer;
            }
        }

        // Collect item content until comma (at depth 0) or stop.
        let item_start = i;
        let mut item_end = i;
        let mut depth = 0usize;

        while item_end < len {
            if !skip_map.is_code(item_end) {
                item_end += 1;
                continue;
            }

            let b = bytes[item_end];

            if b == b'(' {
                depth += 1;
                item_end += 1;
                continue;
            }
            if b == b')' {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                item_end += 1;
                continue;
            }
            if depth == 0 {
                if b == b',' || b == b';' {
                    break;
                }
                let mut stopped = false;
                for &stop in ORDER_BY_STOP {
                    if match_keyword(bytes, skip_map, item_end, stop).is_some() {
                        stopped = true;
                        break;
                    }
                }
                if stopped {
                    break;
                }
            }

            item_end += 1;
        }

        // Determine the last word in the item, skipping NULLS FIRST / NULLS LAST.
        let mut effective_end = item_end;
        let w = last_word(bytes, item_start, effective_end);

        // Check if last word is FIRST or LAST (part of NULLS FIRST/LAST).
        if w.eq_ignore_ascii_case(b"FIRST") || w.eq_ignore_ascii_case(b"LAST") {
            // Strip it off and check the word before it.
            effective_end -= w.len();
            // Skip trailing whitespace before stripped word.
            while effective_end > item_start
                && (bytes[effective_end - 1] == b' '
                    || bytes[effective_end - 1] == b'\t'
                    || bytes[effective_end - 1] == b'\n'
                    || bytes[effective_end - 1] == b'\r')
            {
                effective_end -= 1;
            }
            let w2 = last_word(bytes, item_start, effective_end);
            // If the word before FIRST/LAST is NULLS, strip it too.
            if w2.eq_ignore_ascii_case(b"NULLS") {
                effective_end -= w2.len();
                while effective_end > item_start
                    && (bytes[effective_end - 1] == b' '
                        || bytes[effective_end - 1] == b'\t'
                        || bytes[effective_end - 1] == b'\n'
                        || bytes[effective_end - 1] == b'\r')
                {
                    effective_end -= 1;
                }
            }
        }

        let final_word = last_word(bytes, item_start, effective_end);

        if final_word.eq_ignore_ascii_case(b"ASC") || final_word.eq_ignore_ascii_case(b"DESC") {
            has_explicit = true;
        } else if !final_word.is_empty() {
            has_implicit = true;
        }

        // Advance past comma.
        if item_end < len && bytes[item_end] == b',' {
            i = item_end + 1;
        } else {
            break;
        }
    }

    has_explicit && has_implicit
}
