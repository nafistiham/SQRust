use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct StringLiteralNewline;

impl Rule for StringLiteralNewline {
    fn name(&self) -> &'static str {
        "Ambiguous/StringLiteralNewline"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Detect the opening single-quote of a string literal: it is itself
            // marked as skip, but the byte before it (if any) is code.
            if bytes[i] == b'\'' && (i == 0 || skip.is_code(i - 1)) {
                let str_start = i;
                let content_start = i + 1;
                let mut k = content_start;
                // Walk forward while inside the string (skip bytes).
                while k < len && !skip.is_code(k) {
                    k += 1;
                }
                // Closing quote was at k - 1.
                let content_end = if k > content_start { k - 1 } else { content_start };
                let content = &bytes[content_start..content_end];

                if content.contains(&b'\n') {
                    let (line, col) = offset_to_line_col(source, str_start);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: "String literal contains an actual newline character; use concatenation or a single-line string instead".to_string(),
                        line,
                        col,
                    });
                }

                i = k;
                continue;
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
