use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct CaseWhenSameResult;

impl Rule for CaseWhenSameResult {
    fn name(&self) -> &'static str {
        "Ambiguous/CaseWhenSameResult"
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

            if let Some(after_case) = match_keyword_ci(bytes, len, &skip, i, b"CASE") {
                if let Some(violation_pos) = check_case_expr(bytes, len, &skip, source, after_case) {
                    let (line, col) = offset_to_line_col(source, violation_pos);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: "All CASE branches return the same value — the CASE expression is redundant".to_string(),
                        line,
                        col,
                    });
                }
            }

            i += 1;
        }

        diags
    }
}

/// Checks a CASE expression starting right after the CASE keyword.
/// Returns the offset of CASE (for error reporting) if all branches have the same
/// single-token literal result, else None.
fn check_case_expr(bytes: &[u8], len: usize, skip: &SkipMap, source: &str, after_case: usize) -> Option<usize> {
    let case_kw_start = {
        // Find the actual CASE position (4 bytes before after_case).
        after_case.saturating_sub(4)
    };

    let mut pos = skip_code_whitespace_bytes(bytes, len, after_case);
    let mut branch_values: Vec<String> = Vec::new();
    let mut has_else = false;

    loop {
        pos = skip_code_whitespace_bytes(bytes, len, pos);
        if pos >= len {
            break;
        }

        if let Some(after_when) = match_keyword_ci(bytes, len, skip, pos, b"WHEN") {
            // Skip past the condition until we reach THEN.
            pos = after_when;
            loop {
                pos = skip_code_whitespace_bytes(bytes, len, pos);
                if pos >= len {
                    return None;
                }
                if let Some(after_then) = match_keyword_ci(bytes, len, skip, pos, b"THEN") {
                    pos = after_then;
                    break;
                }
                // Skip one character of the condition.
                pos += 1;
            }

            // Parse the result literal after THEN.
            pos = skip_code_whitespace_bytes(bytes, len, pos);
            match extract_single_token_literal(bytes, len, skip, source, pos) {
                Some((val, end)) => {
                    branch_values.push(val);
                    pos = end;
                }
                None => return None, // complex expression — skip this CASE
            }
        } else if let Some(after_else) = match_keyword_ci(bytes, len, skip, pos, b"ELSE") {
            has_else = true;
            pos = after_else;
            pos = skip_code_whitespace_bytes(bytes, len, pos);

            match extract_single_token_literal(bytes, len, skip, source, pos) {
                Some((val, end)) => {
                    branch_values.push(val);
                    pos = end;
                }
                None => return None,
            }
        } else if match_keyword_ci(bytes, len, skip, pos, b"END").is_some() {
            break;
        } else if skip.is_code(pos) {
            pos += 1;
        } else {
            pos += 1;
        }
    }

    // Need at least 2 branches and they must all be the same value.
    let total = branch_values.len();
    if total < 2 {
        return None;
    }

    // If no ELSE, we need at least 2 WHEN branches.
    if !has_else && total < 2 {
        return None;
    }

    let first = branch_values[0].to_lowercase();
    let all_same = branch_values.iter().all(|v| v.to_lowercase() == first);

    if all_same {
        Some(case_kw_start)
    } else {
        None
    }
}

/// Extracts a single-token literal starting at `pos`.
/// Accepted literals: single-quoted strings, integers, NULL.
/// Returns `(normalized_value, end_offset)` or None if complex.
fn extract_single_token_literal(bytes: &[u8], len: usize, skip: &SkipMap, _source: &str, pos: usize) -> Option<(String, usize)> {
    if pos >= len {
        return None;
    }

    // Single-quoted string — the opening quote is marked as non-code in SkipMap,
    // but we detect it by checking the raw byte before skip classification.
    // Actually the opening quote byte is marked skip=true by SkipMap.build().
    // We need to look at the raw bytes here.
    if bytes[pos] == b'\'' {
        // Find end of string (scan raw bytes).
        let start = pos;
        let mut p = pos + 1;
        while p < len {
            if bytes[p] == b'\'' {
                if p + 1 < len && bytes[p + 1] == b'\'' {
                    p += 2; // escaped quote
                } else {
                    p += 1;
                    break;
                }
            } else {
                p += 1;
            }
        }
        let raw = std::str::from_utf8(&bytes[start..p]).ok()?;
        // Normalize: lowercase the content (strip outer quotes for comparison).
        let inner = &raw[1..raw.len().saturating_sub(1)];
        return Some((inner.to_lowercase(), p));
    }

    // NULL keyword.
    if let Some(after_null) = match_keyword_ci(bytes, len, skip, pos, b"NULL") {
        return Some(("null".to_string(), after_null));
    }

    // Integer (possibly negative).
    let mut p = pos;
    let negative = skip.is_code(p) && bytes[p] == b'-';
    if negative {
        p += 1;
        p = skip_code_whitespace_bytes(bytes, len, p);
    }

    if p < len && skip.is_code(p) && bytes[p].is_ascii_digit() {
        let num_start = if negative { pos } else { p };
        while p < len && skip.is_code(p) && bytes[p].is_ascii_digit() {
            p += 1;
        }
        // Word boundary.
        if p < len && skip.is_code(p) && (bytes[p].is_ascii_alphanumeric() || bytes[p] == b'_') {
            return None;
        }
        let raw = std::str::from_utf8(&bytes[num_start..p]).ok()?;
        return Some((raw.to_string(), p));
    }

    None
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

fn skip_code_whitespace_bytes(bytes: &[u8], len: usize, mut pos: usize) -> usize {
    while pos < len && (bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\n' || bytes[pos] == b'\r') {
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
