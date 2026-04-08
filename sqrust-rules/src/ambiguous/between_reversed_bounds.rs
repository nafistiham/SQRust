use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct BetweenReversedBounds;

impl Rule for BetweenReversedBounds {
    fn name(&self) -> &'static str {
        "Ambiguous/BetweenReversedBounds"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            if !skip.is_code(i) {
                i += 1;
                continue;
            }

            if let Some(after_between) = match_keyword_ci(bytes, len, &skip, i, b"BETWEEN") {
                let after_ws = skip_code_whitespace(bytes, len, &skip, after_between);

                if let Some((low_val, after_low)) = parse_number(bytes, len, &skip, after_ws) {
                    let after_low_ws = skip_code_whitespace(bytes, len, &skip, after_low);

                    if let Some(after_and) = match_keyword_ci(bytes, len, &skip, after_low_ws, b"AND") {
                        let after_and_ws = skip_code_whitespace(bytes, len, &skip, after_and);

                        if let Some((high_val, _)) = parse_number(bytes, len, &skip, after_and_ws) {
                            if low_val > high_val {
                                let (line, col) = offset_to_line_col(source, i);
                                diags.push(Diagnostic {
                                    rule: self.name(),
                                    message: format!(
                                        "BETWEEN bounds appear reversed ({} AND {}) — condition is always false",
                                        low_val, high_val
                                    ),
                                    line,
                                    col,
                                });
                            }
                        }
                    }
                }
            }

            i += 1;
        }

        diags
    }
}

/// Parses a number (optionally negative) starting at `pos`.
/// Returns `(value, end_offset)` if a number was found, else `None`.
/// Supports integers and decimal numbers.
fn parse_number(bytes: &[u8], len: usize, skip: &SkipMap, pos: usize) -> Option<(f64, usize)> {
    if pos >= len || !skip.is_code(pos) {
        return None;
    }

    let mut p = pos;
    let negative = bytes[p] == b'-';
    if negative {
        p += 1;
        // After '-', must have whitespace-skip then a digit.
        p = skip_code_whitespace(bytes, len, skip, p);
    }

    if p >= len || !skip.is_code(p) || !bytes[p].is_ascii_digit() {
        return None;
    }

    let start = p;
    while p < len && skip.is_code(p) && bytes[p].is_ascii_digit() {
        p += 1;
    }

    // Optional decimal part.
    if p < len && skip.is_code(p) && bytes[p] == b'.' {
        p += 1;
        while p < len && skip.is_code(p) && bytes[p].is_ascii_digit() {
            p += 1;
        }
    }

    // Word boundary: must not be followed by alphanumeric or '_'.
    if p < len && skip.is_code(p) && (bytes[p].is_ascii_alphanumeric() || bytes[p] == b'_') {
        return None;
    }

    let num_str = std::str::from_utf8(&bytes[start..p]).ok()?;
    let value: f64 = num_str.parse().ok()?;
    let value = if negative { -value } else { value };

    Some((value, p))
}

fn match_keyword_ci(bytes: &[u8], len: usize, skip: &SkipMap, pos: usize, keyword: &[u8]) -> Option<usize> {
    let kw_len = keyword.len();
    if pos + kw_len > len {
        return None;
    }
    for k in 0..kw_len {
        let b = pos + k;
        if !skip.is_code(b) {
            return None;
        }
        if bytes[b].to_ascii_uppercase() != keyword[k] {
            return None;
        }
    }
    let end = pos + kw_len;
    if end < len && skip.is_code(end) && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        return None;
    }
    Some(end)
}

fn skip_code_whitespace(bytes: &[u8], len: usize, skip: &SkipMap, mut pos: usize) -> usize {
    while pos < len && (bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\n' || bytes[pos] == b'\r') {
        pos += 1;
    }
    // Also skip past any non-code bytes (inside comments/strings that start mid-whitespace).
    while pos < len && !skip.is_code(pos) {
        pos += 1;
    }
    pos
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
