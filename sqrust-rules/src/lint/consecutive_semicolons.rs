use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct ConsecutiveSemicolons;

impl Rule for ConsecutiveSemicolons {
    fn name(&self) -> &'static str {
        "Lint/ConsecutiveSemicolons"
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

            if bytes[i] == b';' {
                let first_semi = i;
                // Scan forward skipping whitespace (including newlines) for another semicolon.
                let mut j = i + 1;
                while j < len
                    && (bytes[j] == b' '
                        || bytes[j] == b'\t'
                        || bytes[j] == b'\n'
                        || bytes[j] == b'\r')
                {
                    j += 1;
                }
                if j < len && skip.is_code(j) && bytes[j] == b';' {
                    let (line, col) = offset_to_line_col(source, first_semi);
                    diags.push(Diagnostic {
                        rule: "Lint/ConsecutiveSemicolons",
                        message: "Consecutive semicolons (;;) found; remove the extra semicolon"
                            .to_string(),
                        line,
                        col,
                    });
                    // Skip past this entire semicolon run to avoid double-counting.
                    i = j;
                    while i < len
                        && (bytes[i] == b';'
                            || bytes[i] == b' '
                            || bytes[i] == b'\t'
                            || bytes[i] == b'\n'
                            || bytes[i] == b'\r')
                    {
                        i += 1;
                    }
                    continue;
                }
            }
            i += 1;
        }

        diags
    }
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
