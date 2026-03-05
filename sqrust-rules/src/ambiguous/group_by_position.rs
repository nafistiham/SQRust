use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::SkipMap;

pub struct GroupByPosition;

impl Rule for GroupByPosition {
    fn name(&self) -> &'static str {
        "Ambiguous/GroupByPosition"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Skip non-code positions (strings, comments).
            if !skip_map.is_code(i) {
                i += 1;
                continue;
            }

            // Try to match the keyword GROUP at a word boundary.
            if let Some(after_group) = match_keyword(bytes, &skip_map, i, b"GROUP") {
                // Skip whitespace/newlines between GROUP and BY.
                let after_ws = skip_whitespace(bytes, after_group);

                // Try to match BY.
                if let Some(after_by) = match_keyword(bytes, &skip_map, after_ws, b"BY") {
                    // We found GROUP BY — scan the comma-separated list.
                    scan_positional_list(
                        bytes,
                        &skip_map,
                        source,
                        after_by,
                        self.name(),
                        "Avoid positional GROUP BY references; use column names",
                        GROUP_BY_STOP_KEYWORDS,
                        &mut diags,
                    );
                    i = after_by;
                    continue;
                }
            }

            i += 1;
        }

        diags
    }
}

/// Keywords that terminate a GROUP BY item list.
const GROUP_BY_STOP_KEYWORDS: &[&[u8]] = &[
    b"HAVING", b"ORDER", b"LIMIT", b"UNION", b"INTERSECT", b"EXCEPT",
];

/// Keywords that terminate an ORDER BY item list.
pub(super) const ORDER_BY_STOP_KEYWORDS: &[&[u8]] = &[
    b"LIMIT", b"UNION", b"INTERSECT", b"EXCEPT",
];

/// Attempts to match `keyword` (case-insensitive, ASCII) at position `i` in `bytes`,
/// requiring:
/// - `i` is at a code position
/// - the character before `i` is not a word character (word boundary start)
/// - the character after the keyword is not a word character (word boundary end)
///
/// Returns the byte offset just after the keyword if matched, or None.
pub(super) fn match_keyword(
    bytes: &[u8],
    skip_map: &SkipMap,
    i: usize,
    keyword: &[u8],
) -> Option<usize> {
    let len = bytes.len();
    let klen = keyword.len();

    if i + klen > len {
        return None;
    }

    // Must start at a code position.
    if !skip_map.is_code(i) {
        return None;
    }

    // Word boundary before: not preceded by a word character.
    if i > 0 && is_word_char(bytes[i - 1]) {
        return None;
    }

    // Case-insensitive match; every keyword byte must be code.
    for k in 0..klen {
        if !bytes[i + k].eq_ignore_ascii_case(&keyword[k]) {
            return None;
        }
        if !skip_map.is_code(i + k) {
            return None;
        }
    }

    // Word boundary after: not followed by a word character.
    let end = i + klen;
    if end < len && is_word_char(bytes[end]) {
        return None;
    }

    Some(end)
}

/// Skips ASCII whitespace (space, tab, newline, carriage return).
pub(super) fn skip_whitespace(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len()
        && (bytes[i] == b' '
            || bytes[i] == b'\t'
            || bytes[i] == b'\n'
            || bytes[i] == b'\r')
    {
        i += 1;
    }
    i
}

/// Returns true if `ch` is a word character (letter, digit, underscore).
#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Converts a byte offset to 1-indexed (line, col).
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Returns `true` if the slice is a bare integer literal (all ASCII digits, non-empty).
fn is_bare_integer(slice: &[u8]) -> bool {
    !slice.is_empty() && slice.iter().all(|b| b.is_ascii_digit())
}

/// Trims leading ASCII whitespace from a byte slice.
fn trim_leading(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .take_while(|&&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r')
        .count();
    &bytes[start..]
}

/// Trims trailing ASCII whitespace from a byte slice.
fn trim_trailing(bytes: &[u8]) -> &[u8] {
    let end = bytes
        .iter()
        .rposition(|&b| b != b' ' && b != b'\t' && b != b'\n' && b != b'\r')
        .map(|p| p + 1)
        .unwrap_or(0);
    &bytes[..end]
}

/// Strips a trailing `ASC` or `DESC` token (case-insensitive) from item bytes.
/// Only strips if the last word is exactly ASC or DESC.
fn strip_trailing_direction(bytes: &[u8]) -> &[u8] {
    let trimmed = trim_trailing(bytes);
    let word_end = trimmed.len();
    if word_end == 0 {
        return trimmed;
    }
    // Walk backwards over word characters.
    let mut word_start = word_end;
    while word_start > 0 && is_word_char(trimmed[word_start - 1]) {
        word_start -= 1;
    }
    let last_word = &trimmed[word_start..word_end];
    if (last_word.eq_ignore_ascii_case(b"ASC") || last_word.eq_ignore_ascii_case(b"DESC"))
        && word_start > 0
    {
        trim_trailing(&trimmed[..word_start])
    } else {
        trimmed
    }
}

/// Locates the first occurrence of `first_byte` in `bytes[search_start..]` and
/// returns its absolute offset. Used to find the exact start of an integer token.
fn find_first_byte_offset(bytes: &[u8], search_start: usize, first_byte: u8) -> usize {
    let mut i = search_start;
    while i < bytes.len() {
        if bytes[i] == first_byte {
            return i;
        }
        i += 1;
    }
    search_start
}

/// Scans the comma-separated expression list that follows GROUP BY or ORDER BY.
///
/// For each item that reduces to a bare integer at a code position, emits a Diagnostic.
///
/// - `start`: byte offset immediately after the `BY` keyword
/// - `rule_name`: the `&'static str` rule name for diagnostics
/// - `message`: the violation message to embed in each Diagnostic
/// - `stop_keywords`: keywords that terminate the clause
/// - `diags`: output vector
pub(super) fn scan_positional_list(
    bytes: &[u8],
    skip_map: &SkipMap,
    source: &str,
    start: usize,
    rule_name: &'static str,
    message: &'static str,
    stop_keywords: &[&[u8]],
    diags: &mut Vec<Diagnostic>,
) {
    let len = bytes.len();
    let mut i = start;

    'outer: loop {
        // Skip leading whitespace before the next item.
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

        // Semicolon always terminates.
        if skip_map.is_code(i) && bytes[i] == b';' {
            break;
        }

        // Check if we've hit a stop keyword (end of clause).
        for &stop in stop_keywords {
            if match_keyword(bytes, skip_map, i, stop).is_some() {
                break 'outer;
            }
        }

        // Record where this item begins (in absolute source offsets).
        let item_start_abs = i;

        // Collect item content until ',' or stop.
        let mut item_end_abs = i;
        while item_end_abs < len {
            if !skip_map.is_code(item_end_abs) {
                item_end_abs += 1;
                continue;
            }

            if bytes[item_end_abs] == b',' || bytes[item_end_abs] == b';' {
                break;
            }

            let mut stopped = false;
            for &stop in stop_keywords {
                if match_keyword(bytes, skip_map, item_end_abs, stop).is_some() {
                    stopped = true;
                    break;
                }
            }
            if stopped {
                break;
            }

            item_end_abs += 1;
        }

        // Slice the item from the source bytes.
        let item_slice = &bytes[item_start_abs..item_end_abs];

        // Trim, strip direction keyword, trim again.
        let trimmed = trim_leading(item_slice);
        let trimmed = strip_trailing_direction(trimmed);
        let trimmed = trim_trailing(trimmed);

        if !trimmed.is_empty() && is_bare_integer(trimmed) {
            // The trimmed content starts with a digit.  Find the exact offset
            // in the original source of that first digit.
            let first_digit = trimmed[0];
            let int_abs = find_first_byte_offset(bytes, item_start_abs, first_digit);
            let (line, col) = line_col(source, int_abs);
            diags.push(Diagnostic {
                rule: rule_name,
                message: message.to_string(),
                line,
                col,
            });
        }

        // Advance past comma (if any) to next item.
        if item_end_abs < len && bytes[item_end_abs] == b',' {
            i = item_end_abs + 1;
        } else {
            break;
        }
    }
}
