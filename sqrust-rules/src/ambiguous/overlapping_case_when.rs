use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct OverlappingCaseWhen;

impl Rule for OverlappingCaseWhen {
    fn name(&self) -> &'static str {
        "Ambiguous/OverlappingCaseWhen"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Only examine code bytes (not inside strings/comments).
            if !skip.is_code(i) {
                i += 1;
                continue;
            }

            // Try to match `CASE` (case-insensitive) at position i.
            if let Some(after_case) = match_keyword_ci(bytes, len, &skip, i, b"CASE") {
                // Skip whitespace after CASE.
                let after_case_ws = skip_code_whitespace(bytes, len, &skip, after_case);

                // Try to match `WHEN`.
                if let Some(after_when) = match_keyword_ci(bytes, len, &skip, after_case_ws, b"WHEN") {
                    // Skip whitespace after WHEN.
                    let after_when_ws = skip_code_whitespace(bytes, len, &skip, after_when);

                    // Check for always-true conditions.
                    if is_always_true_condition(bytes, len, &skip, after_when_ws) {
                        let (line, col) = offset_to_line_col(source, i);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: "CASE WHEN condition is always TRUE, making subsequent WHEN branches unreachable".to_string(),
                            line,
                            col,
                        });
                    }
                }
            }

            i += 1;
        }

        diags
    }
}

/// Attempts to match `keyword` (ASCII, case-insensitive) at byte offset `pos`
/// in `bytes`, considering only code positions (not inside strings/comments).
/// Returns the offset immediately after the keyword if matched, else `None`.
/// The keyword must be followed by a non-alphanumeric code byte (word boundary).
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
    // Word boundary: next byte must be non-alphanumeric or end of input.
    let end = pos + kw_len;
    if end < len && skip.is_code(end) && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        return None;
    }
    Some(end)
}

/// Skips whitespace characters that are code (not in strings/comments).
fn skip_code_whitespace(bytes: &[u8], len: usize, skip: &SkipMap, mut pos: usize) -> usize {
    while pos < len && skip.is_code(pos) && (bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\n' || bytes[pos] == b'\r') {
        pos += 1;
    }
    pos
}

/// Returns `true` if the content at `pos` matches an always-true WHEN condition
/// followed by a THEN keyword (with optional whitespace).
/// Patterns:
///   - `TRUE THEN`
///   - `1=1 THEN` (with optional spaces around =)
fn is_always_true_condition(bytes: &[u8], len: usize, skip: &SkipMap, pos: usize) -> bool {
    // Try `TRUE`
    if let Some(after_true) = match_keyword_ci(bytes, len, skip, pos, b"TRUE") {
        let after_ws = skip_code_whitespace(bytes, len, skip, after_true);
        if match_keyword_ci(bytes, len, skip, after_ws, b"THEN").is_some() {
            return true;
        }
    }

    // Try `1 = 1` or `1=1`
    if pos < len && skip.is_code(pos) && bytes[pos] == b'1' {
        // Check it's not followed by more digits (i.e., it's the literal 1, not 10, 11, etc.)
        let after_1 = pos + 1;
        let next_is_non_digit = after_1 >= len
            || !skip.is_code(after_1)
            || !bytes[after_1].is_ascii_digit();
        if next_is_non_digit {
            let after_ws1 = skip_code_whitespace(bytes, len, skip, after_1);
            // Expect `=`
            if after_ws1 < len && skip.is_code(after_ws1) && bytes[after_ws1] == b'=' {
                let after_eq = after_ws1 + 1;
                let after_ws2 = skip_code_whitespace(bytes, len, skip, after_eq);
                // Expect `1`
                if after_ws2 < len && skip.is_code(after_ws2) && bytes[after_ws2] == b'1' {
                    let after_rhs = after_ws2 + 1;
                    let rhs_non_digit = after_rhs >= len
                        || !skip.is_code(after_rhs)
                        || !bytes[after_rhs].is_ascii_digit();
                    if rhs_non_digit {
                        let after_ws3 = skip_code_whitespace(bytes, len, skip, after_rhs);
                        if match_keyword_ci(bytes, len, skip, after_ws3, b"THEN").is_some() {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
