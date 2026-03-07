use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::SkipMap;

pub struct LeadingZeroNumeric;

/// Characters that can immediately precede a bare `.N` numeric literal.
/// A dot preceded by anything else (letter, digit, `_`) is not a numeric literal.
const TRIGGER_CHARS: &[u8] = &[
    b' ', b'\t', b'\n', b'\r', // whitespace
    b'(', b')', b'[', b']',    // brackets
    b'=', b'<', b'>',          // comparison operators
    b'+', b'-', b'*', b'/',    // arithmetic operators
    b',', b';',                // punctuation
    b'!',                      // negation (for !=)
];

impl Rule for LeadingZeroNumeric {
    fn name(&self) -> &'static str {
        "Convention/LeadingZeroNumeric"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();

        for i in 0..len {
            // Only consider '.' characters in code (not inside strings/comments).
            if bytes[i] != b'.' || !skip_map.is_code(i) {
                continue;
            }

            // The character after '.' must be a digit.
            if i + 1 >= len || !bytes[i + 1].is_ascii_digit() {
                continue;
            }

            // The character before '.' must be a trigger character or we are
            // at the start of file. A letter, digit, or underscore before '.'
            // means this is a qualified identifier (t.col) or a decimal number
            // (1.5), neither of which should be flagged.
            let preceded_by_trigger = if i == 0 {
                true
            } else {
                let prev = bytes[i - 1];
                TRIGGER_CHARS.contains(&prev)
            };

            if !preceded_by_trigger {
                continue;
            }

            let (line, col) = line_col(source, i);
            diags.push(Diagnostic {
                rule: self.name(),
                message: "Numeric literal missing leading zero (e.g. use 0.5 instead of .5)"
                    .to_string(),
                line,
                col,
            });
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
