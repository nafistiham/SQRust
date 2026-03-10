use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct ArithmeticOperatorPadding;

impl Rule for ArithmeticOperatorPadding {
    fn name(&self) -> &'static str {
        "Layout/ArithmeticOperatorPadding"
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

            let op = bytes[i];
            if op == b'+' || op == b'-' || op == b'*' || op == b'/' || op == b'%' {
                // SkipMap already marks comment bytes as non-code; the outer guard
                // `if !skip.is_code(i)` ensures we never reach this point for `--`
                // or `/* */` comment bytes. No manual comment skip needed here.

                // * inside parentheses: SELECT *, COUNT(*) etc.
                if op == b'*' {
                    let prev_nws = prev_non_whitespace(bytes, i);
                    let next_nws = next_non_whitespace(bytes, i, len);
                    if prev_nws == Some(b'(') || next_nws == Some(b')') {
                        i += 1;
                        continue;
                    }
                }
                // Unary +/- after (, =, >, <, !, ,
                if op == b'+' || op == b'-' {
                    let prev = prev_non_whitespace(bytes, i);
                    match prev {
                        None | Some(b'(') | Some(b'=') | Some(b'>') | Some(b'<') | Some(b'!') | Some(b',') => {
                            i += 1;
                            continue;
                        }
                        _ => {}
                    }
                }

                // Check padding: need space before AND after
                let space_before = i == 0 || is_space(bytes[i - 1]);
                let space_after = i + 1 >= len || is_space(bytes[i + 1]);

                if !space_before || !space_after {
                    let (line, col) = offset_to_line_col(source, i);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: format!(
                            "Arithmetic operator '{}' must be padded with spaces on both sides",
                            bytes[i] as char
                        ),
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

fn is_space(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
}

fn prev_non_whitespace(bytes: &[u8], pos: usize) -> Option<u8> {
    if pos == 0 { return None; }
    let mut j = pos - 1;
    loop {
        if !is_space(bytes[j]) {
            return Some(bytes[j]);
        }
        if j == 0 { return None; }
        j -= 1;
    }
}

fn next_non_whitespace(bytes: &[u8], pos: usize, len: usize) -> Option<u8> {
    if pos + 1 >= len { return None; }
    let mut j = pos + 1;
    while j < len {
        if !is_space(bytes[j]) {
            return Some(bytes[j]);
        }
        j += 1;
    }
    None
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
