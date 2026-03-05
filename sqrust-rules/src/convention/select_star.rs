use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::SkipMap;

pub struct SelectStar;

impl Rule for SelectStar {
    fn name(&self) -> &'static str {
        "Convention/SelectStar"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();

        for i in 0..len {
            if bytes[i] != b'*' {
                continue;
            }
            if !skip_map.is_code(i) {
                continue;
            }

            // Case 1: qualified wildcard — immediately preceded by '.'
            // e.g. `t.*`
            if i > 0 && bytes[i - 1] == b'.' {
                let (line, col) = line_col(source, i);
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "Avoid SELECT *; list columns explicitly".to_string(),
                    line,
                    col,
                });
                continue;
            }

            // Case 2: standalone wildcard — preceded by whitespace AND not
            // preceded by '(' (which would make it COUNT(*) style).
            // The '*' must also be followed by whitespace, ',', ';', or EOF.
            if i > 0 {
                let prev = bytes[i - 1];
                if prev == b'(' {
                    // Inside a function call: COUNT(*), SUM(*), etc. — skip.
                    continue;
                }
                if prev == b' ' || prev == b'\t' || prev == b'\n' || prev == b'\r' {
                    // Check what follows the '*'
                    let next = if i + 1 < len { bytes[i + 1] } else { 0 };
                    let followed_by_separator = next == b' '
                        || next == b'\t'
                        || next == b'\n'
                        || next == b'\r'
                        || next == b','
                        || next == b';'
                        || next == 0;
                    if followed_by_separator {
                        let (line, col) = line_col(source, i);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: "Avoid SELECT *; list columns explicitly".to_string(),
                            line,
                            col,
                        });
                    }
                }
            }
        }

        diags
    }
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
