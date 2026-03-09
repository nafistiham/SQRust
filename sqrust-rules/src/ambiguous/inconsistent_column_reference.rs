use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::SkipMap;
use super::group_by_position::{match_keyword, skip_whitespace};

pub struct InconsistentColumnReference;

impl Rule for InconsistentColumnReference {
    fn name(&self) -> &'static str {
        "Ambiguous/InconsistentColumnReference"
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

            // Check ORDER BY.
            if let Some(after_order) = match_keyword(bytes, &skip_map, i, b"ORDER") {
                let after_ws = skip_whitespace(bytes, after_order);
                if let Some(after_by) = match_keyword(bytes, &skip_map, after_ws, b"BY") {
                    if has_mixed_refs(bytes, &skip_map, after_by, ORDER_BY_STOP) {
                        let (line, col) = offset_to_line_col(source, i);
                        diags.push(Diagnostic {
                            rule: "Ambiguous/InconsistentColumnReference",
                            message: "ORDER BY mixes positional column references (e.g. 1) with named references; use one style consistently".to_string(),
                            line,
                            col,
                        });
                    }
                    i = after_by;
                    continue;
                }
            }

            // Check GROUP BY.
            if let Some(after_group) = match_keyword(bytes, &skip_map, i, b"GROUP") {
                let after_ws = skip_whitespace(bytes, after_group);
                if let Some(after_by) = match_keyword(bytes, &skip_map, after_ws, b"BY") {
                    if has_mixed_refs(bytes, &skip_map, after_by, GROUP_BY_STOP) {
                        let (line, col) = offset_to_line_col(source, i);
                        diags.push(Diagnostic {
                            rule: "Ambiguous/InconsistentColumnReference",
                            message: "GROUP BY mixes positional column references (e.g. 1) with named references; use one style consistently".to_string(),
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

/// Stop keywords that terminate a GROUP BY item list.
const GROUP_BY_STOP: &[&[u8]] = &[
    b"HAVING", b"ORDER", b"LIMIT", b"UNION", b"INTERSECT", b"EXCEPT",
];

/// Converts a byte offset to 1-indexed (line, col).
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Returns `true` if `ch` is an ASCII digit.
#[inline]
fn is_digit(ch: u8) -> bool {
    ch.is_ascii_digit()
}

/// Returns `true` if `ch` is a word character (`[a-zA-Z0-9_]`).
#[inline]
fn is_word_start(ch: u8) -> bool {
    ch.is_ascii_alphabetic() || ch == b'_' || ch == b'"' || ch == b'`'
}

/// Scans the comma-separated expression list that follows GROUP BY or ORDER BY.
/// Returns true if the clause mixes positional (integer) and named references.
fn has_mixed_refs(
    bytes: &[u8],
    skip_map: &SkipMap,
    start: usize,
    stop_keywords: &[&[u8]],
) -> bool {
    let len = bytes.len();
    let mut i = start;
    let mut has_positional = false;
    let mut has_named = false;

    'outer: loop {
        // Skip leading whitespace.
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
        for &stop in stop_keywords {
            if match_keyword(bytes, skip_map, i, stop).is_some() {
                break 'outer;
            }
        }

        // Find the first significant code token in this item.
        // Collect item until next comma at depth 0 or stop.
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
                for &stop in stop_keywords {
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

        // Inspect the first significant code token in this item.
        let mut j = item_start;
        // Skip leading whitespace inside item.
        while j < item_end
            && (bytes[j] == b' '
                || bytes[j] == b'\t'
                || bytes[j] == b'\n'
                || bytes[j] == b'\r')
        {
            j += 1;
        }

        if j < item_end && skip_map.is_code(j) {
            let ch = bytes[j];
            if is_digit(ch) {
                // Check it's a pure integer token (all digits, not e.g. 1+expr).
                let mut k = j;
                while k < item_end && skip_map.is_code(k) && bytes[k].is_ascii_digit() {
                    k += 1;
                }
                // After digits there should be whitespace, comma or end-of-item for a positional ref.
                let next_code = {
                    let mut n = k;
                    while n < item_end
                        && (bytes[n] == b' '
                            || bytes[n] == b'\t'
                            || bytes[n] == b'\n'
                            || bytes[n] == b'\r')
                    {
                        n += 1;
                    }
                    n
                };
                // If after the integer there's nothing meaningful in this item (possibly ASC/DESC), it's positional.
                let after_int_word: &[u8] = if next_code < item_end {
                    let word_start = next_code;
                    let mut word_end = next_code;
                    while word_end < item_end && skip_map.is_code(word_end) && (bytes[word_end].is_ascii_alphanumeric() || bytes[word_end] == b'_') {
                        word_end += 1;
                    }
                    &bytes[word_start..word_end]
                } else {
                    &[]
                };
                // Only count as positional if the token after the digits is ASC, DESC, NULLS, or end.
                if after_int_word.is_empty()
                    || after_int_word.eq_ignore_ascii_case(b"ASC")
                    || after_int_word.eq_ignore_ascii_case(b"DESC")
                    || after_int_word.eq_ignore_ascii_case(b"NULLS")
                {
                    has_positional = true;
                } else {
                    // e.g. `1 + col` — treat as named/expression
                    has_named = true;
                }
            } else if is_word_start(ch) {
                has_named = true;
            }
        }

        // Advance past comma.
        if item_end < len && bytes[item_end] == b',' {
            i = item_end + 1;
        } else {
            break;
        }
    }

    has_positional && has_named
}
