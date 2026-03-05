use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct SingleSpaceAfterComma;

impl Rule for SingleSpaceAfterComma {
    fn name(&self) -> &'static str {
        "Layout/SingleSpaceAfterComma"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = ctx.source.as_bytes();
        let len = source.len();
        if len == 0 {
            return Vec::new();
        }

        let skip_map = SkipMap::build(&ctx.source);
        let mut diags = Vec::new();

        for i in 0..len {
            if source[i] != b',' {
                continue;
            }
            if !skip_map.is_code(i) {
                continue;
            }

            // Determine what follows the comma
            let next = if i + 1 < len { Some(source[i + 1]) } else { None };

            let bad = match next {
                // End of file with no following character — not a violation
                None => false,
                // Trailing comma at end of line — OK
                Some(b'\n') | Some(b'\r') => false,
                // Exactly one space — only bad if the character after that space is also a space
                Some(b' ') => {
                    // Check for double space (extra space)
                    matches!(source.get(i + 2), Some(b' '))
                }
                // Any other character (non-space, non-newline) — missing space
                Some(_) => true,
            };

            if bad {
                let (line, col) = byte_offset_to_line_col(&ctx.source, i);
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "Expected single space after comma".to_string(),
                    line,
                    col,
                });
            }
        }

        diags
    }
}

/// Converts a byte offset into a 1-indexed (line, col) pair.
/// Col counts bytes (ASCII SQL is byte == char for identifiers).
fn byte_offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut line_start = 0usize;
    for (i, ch) in source.char_indices() {
        if i == offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let col = offset - line_start + 1;
    (line, col)
}
