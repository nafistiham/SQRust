use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::SkipMap;
use super::group_by_position::{match_keyword, skip_whitespace};

pub struct ImplicitOrderDirection;

impl Rule for ImplicitOrderDirection {
    fn name(&self) -> &'static str {
        "Ambiguous/ImplicitOrderDirection"
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
                    check_order_by_items(bytes, &skip_map, source, after_by, &mut diags);
                    i = after_by;
                    continue;
                }
            }

            i += 1;
        }

        diags
    }
}

/// Keywords that terminate an ORDER BY item list.
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

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Returns the last word token in `bytes[start..end]`, scanning backwards,
/// ignoring trailing whitespace.
fn last_word(bytes: &[u8], start: usize, end: usize) -> &[u8] {
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
    let word_end = e;
    let mut word_start = e;
    while word_start > start && is_word_char(bytes[word_start - 1]) {
        word_start -= 1;
    }
    &bytes[word_start..word_end]
}

/// Strip NULLS FIRST / NULLS LAST from the end of the effective item range.
/// Returns the adjusted end offset.
fn strip_nulls_clause(bytes: &[u8], start: usize, end: usize) -> usize {
    let w = last_word(bytes, start, end);
    if !w.eq_ignore_ascii_case(b"FIRST") && !w.eq_ignore_ascii_case(b"LAST") {
        return end;
    }
    // Strip the FIRST/LAST word.
    let mut e = end - w.len();
    while e > start
        && (bytes[e - 1] == b' '
            || bytes[e - 1] == b'\t'
            || bytes[e - 1] == b'\n'
            || bytes[e - 1] == b'\r')
    {
        e -= 1;
    }
    let w2 = last_word(bytes, start, e);
    if w2.eq_ignore_ascii_case(b"NULLS") {
        e -= w2.len();
        while e > start
            && (bytes[e - 1] == b' '
                || bytes[e - 1] == b'\t'
                || bytes[e - 1] == b'\n'
                || bytes[e - 1] == b'\r')
        {
            e -= 1;
        }
    }
    e
}

/// Check each ORDER BY item and emit a diagnostic if no explicit ASC/DESC is given.
fn check_order_by_items(
    bytes: &[u8],
    skip_map: &SkipMap,
    source: &str,
    start: usize,
    diags: &mut Vec<Diagnostic>,
) {
    let len = bytes.len();
    let mut i = start;

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

        // Record item start position (for diagnostic location).
        let item_start = i;

        // Collect item content until comma (at depth 0) or stop.
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

        // Strip NULLS FIRST / NULLS LAST, then check last word.
        let effective_end = strip_nulls_clause(bytes, item_start, item_end);
        let final_word = last_word(bytes, item_start, effective_end);

        // If the item is non-empty and doesn't end in ASC or DESC, it's implicit.
        if !final_word.is_empty()
            && !final_word.eq_ignore_ascii_case(b"ASC")
            && !final_word.eq_ignore_ascii_case(b"DESC")
        {
            // Report at the position of the ORDER BY keyword (item_start after whitespace trim).
            let (line, col) = offset_to_line_col(source, item_start);
            diags.push(Diagnostic {
                rule: "Ambiguous/ImplicitOrderDirection",
                message: "ORDER BY expression has no explicit direction — add ASC or DESC to make sort order unambiguous".to_string(),
                line,
                col,
            });
        }

        // Advance past comma.
        if item_end < len && bytes[item_end] == b',' {
            i = item_end + 1;
        } else {
            break;
        }
    }
}
