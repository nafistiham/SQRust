use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct SelectTargetNewLine;

impl Rule for SelectTargetNewLine {
    fn name(&self) -> &'static str {
        "Layout/SelectTargetNewLine"
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

            if let Some(after_select) = match_kw(bytes, &skip_map, i, b"SELECT") {
                // Skip DISTINCT or ALL.
                let mut pos = skip_ws(bytes, after_select);
                if let Some(p) = match_kw(bytes, &skip_map, pos, b"DISTINCT") {
                    pos = skip_ws(bytes, p);
                } else if let Some(p) = match_kw(bytes, &skip_map, pos, b"ALL") {
                    pos = skip_ws(bytes, p);
                }

                if let Some(violation_pos) = scan_select_targets(bytes, &skip_map, pos) {
                    let (line, col) = offset_to_line_col(source, violation_pos);
                    diags.push(Diagnostic {
                        rule: "Layout/SelectTargetNewLine",
                        message: "Multiple SELECT columns on the same line; put each column on its own line".to_string(),
                        line,
                        col,
                    });
                }
                i = after_select;
                continue;
            }

            i += 1;
        }

        diags
    }
}

/// Scans the SELECT column list starting at `scan_start`.
/// Returns `Some(pos)` at the position of the first column that is not on its
/// own line (i.e., two consecutive columns are not separated by a newline).
/// Returns `None` if every column separation is newline-delimited.
///
/// The check per top-level comma: after the comma, is there a newline before
/// the next non-whitespace, non-skip code byte? If not → violation at the
/// position right after the comma (the next column starts there on the same line).
fn scan_select_targets(bytes: &[u8], skip_map: &SkipMap, scan_start: usize) -> Option<usize> {
    let len = bytes.len();
    let stop_kws: &[&[u8]] = &[
        b"FROM", b"WHERE", b"GROUP", b"ORDER", b"HAVING", b"LIMIT",
        b"UNION", b"INTERSECT", b"EXCEPT", b"FETCH",
    ];
    let mut i = scan_start;
    let mut depth = 0i32;
    let mut comma_count = 0usize;

    while i < len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }
        let b = bytes[i];

        if b == b'(' {
            depth += 1;
            i += 1;
            continue;
        }
        if b == b')' {
            if depth > 0 {
                depth -= 1;
                i += 1;
                continue;
            } else {
                break;
            }
        }

        if depth == 0 {
            // Check stop keywords.
            let mut at_stop = false;
            for kw in stop_kws {
                if match_kw(bytes, skip_map, i, kw).is_some() {
                    at_stop = true;
                    break;
                }
            }
            if at_stop || b == b';' {
                break;
            }

            if b == b',' {
                comma_count += 1;
                // After this comma, check if there is a newline before the next column.
                // Scan forward through whitespace and skip-map bytes; if we reach
                // a code byte without passing through a '\n', it's a violation.
                let comma_pos = i;
                let mut j = i + 1;
                let mut found_newline = false;
                while j < len {
                    if bytes[j] == b'\n' {
                        found_newline = true;
                        break;
                    }
                    // If we hit a non-whitespace code byte before a newline, violation.
                    if skip_map.is_code(j)
                        && bytes[j] != b' '
                        && bytes[j] != b'\t'
                        && bytes[j] != b'\r'
                    {
                        // Also check if we're at a stop keyword — if so, no more columns.
                        let mut at_stop2 = false;
                        for kw in stop_kws {
                            if match_kw(bytes, skip_map, j, kw).is_some() {
                                at_stop2 = true;
                                break;
                            }
                        }
                        if !at_stop2 && bytes[j] != b';' {
                            return Some(comma_pos + 1);
                        }
                        break;
                    }
                    j += 1;
                }
                if found_newline {
                    i += 1;
                    continue;
                }
                i += 1;
                continue;
            }
        }

        i += 1;
    }

    // If we found commas but no violations above, also check that the
    // first column is not on the same line as the second. The comma-after
    // check above handles this since it checks from AFTER each comma.
    // Return None if no violation found.
    let _ = comma_count;
    None
}

fn match_kw(bytes: &[u8], skip_map: &SkipMap, i: usize, kw: &[u8]) -> Option<usize> {
    let len = bytes.len();
    let kw_len = kw.len();
    if i + kw_len > len {
        return None;
    }
    if !skip_map.is_code(i) {
        return None;
    }
    let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
    if !before_ok {
        return None;
    }
    let matches = bytes[i..i + kw_len]
        .iter()
        .zip(kw.iter())
        .all(|(&a, &b)| a.to_ascii_uppercase() == b.to_ascii_uppercase());
    if !matches {
        return None;
    }
    let end = i + kw_len;
    if end < len && is_word_char(bytes[end]) {
        return None;
    }
    Some(end)
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len()
        && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r')
    {
        i += 1;
    }
    i
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
