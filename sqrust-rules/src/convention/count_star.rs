use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct CountStar;

impl Rule for CountStar {
    fn name(&self) -> &'static str {
        "Convention/CountStar"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();

        // Pattern: <non-word-char> COUNT ( 1 )
        // We scan for 'C'/'c' and attempt to match COUNT(1) at that position.
        let mut i = 0;
        while i < len {
            // Quick pre-check: byte must be 'C' or 'c'
            if bytes[i] != b'C' && bytes[i] != b'c' {
                i += 1;
                continue;
            }

            // Word boundary before: preceding byte must not be a word char
            if i > 0 && is_word_char(bytes[i - 1]) {
                i += 1;
                continue;
            }

            // Must have at least 8 bytes for "COUNT(1)" (5 + 1 + 1 + 1)
            if i + 7 >= len {
                i += 1;
                continue;
            }

            // Match C O U N T (case-insensitive)
            let is_count = bytes[i].eq_ignore_ascii_case(&b'C')
                && bytes[i + 1].eq_ignore_ascii_case(&b'O')
                && bytes[i + 2].eq_ignore_ascii_case(&b'U')
                && bytes[i + 3].eq_ignore_ascii_case(&b'N')
                && bytes[i + 4].eq_ignore_ascii_case(&b'T');
            if !is_count {
                i += 1;
                continue;
            }

            // Next char after COUNT must be '(' with no word char after COUNT
            // (i.e. COUNT must end the word — not COUNTx)
            if is_word_char(bytes[i + 5]) {
                // e.g. COUNTER — skip
                i += 1;
                continue;
            }

            // Must be followed immediately by '('
            if bytes[i + 5] != b'(' {
                i += 1;
                continue;
            }

            // Then exactly '1'
            if bytes[i + 6] != b'1' {
                i += 1;
                continue;
            }

            // Then ')'
            if bytes[i + 7] != b')' {
                i += 1;
                continue;
            }

            // The '1' at i+6 must be at a code position (not in string/comment)
            if !skip_map.is_code(i + 6) {
                i += 1;
                continue;
            }

            // The 'C' at i must also be at a code position
            if !skip_map.is_code(i) {
                i += 1;
                continue;
            }

            let (line, col) = line_col(source, i);
            diags.push(Diagnostic {
                rule: self.name(),
                message: "Use COUNT(*) instead of COUNT(1)".to_string(),
                line,
                col,
            });

            // Advance past COUNT(1)
            i += 8;
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
