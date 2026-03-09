use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct SetOperatorNewLine;

impl Rule for SetOperatorNewLine {
    fn name(&self) -> &'static str {
        "Layout/SetOperatorNewLine"
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

            if let Some((kw_start, kw_end)) = try_set_op(bytes, &skip_map, i) {
                // Skip ALL or DISTINCT after operator (horizontal whitespace only).
                let mut after_kw = skip_ws_h(bytes, kw_end);
                if let Some(e) = match_kw(bytes, &skip_map, after_kw, b"ALL") {
                    after_kw = skip_ws_h(bytes, e);
                } else if let Some(e) = match_kw(bytes, &skip_map, after_kw, b"DISTINCT") {
                    after_kw = skip_ws_h(bytes, e);
                }

                // Check: only whitespace before kw_start on same line?
                let newline_before = only_ws_before_on_line(bytes, kw_start);
                // Check: at after_kw, is there a newline, EOF, or line comment?
                let newline_after = after_kw >= len
                    || bytes[after_kw] == b'\n'
                    || bytes[after_kw] == b'\r'
                    || (after_kw + 1 < len
                        && bytes[after_kw] == b'-'
                        && bytes[after_kw + 1] == b'-');

                if !newline_before || !newline_after {
                    let (line, col) = offset_to_line_col(source, kw_start);
                    diags.push(Diagnostic {
                        rule: "Layout/SetOperatorNewLine",
                        message: "Set operator (UNION/INTERSECT/EXCEPT) must be on its own line, surrounded by newlines".to_string(),
                        line,
                        col,
                    });
                }

                i = kw_end;
                continue;
            }

            i += 1;
        }

        diags
    }
}

fn try_set_op(bytes: &[u8], skip_map: &SkipMap, i: usize) -> Option<(usize, usize)> {
    for kw in &[b"UNION" as &[u8], b"INTERSECT", b"EXCEPT"] {
        if let Some(end) = match_kw(bytes, skip_map, i, kw) {
            return Some((i, end));
        }
    }
    None
}

fn only_ws_before_on_line(bytes: &[u8], i: usize) -> bool {
    let mut j = i;
    loop {
        if j == 0 {
            return true;
        }
        j -= 1;
        if bytes[j] == b'\n' {
            return true;
        }
        if bytes[j] != b' ' && bytes[j] != b'\t' {
            return false;
        }
    }
}

fn skip_ws_h(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    i
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

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
