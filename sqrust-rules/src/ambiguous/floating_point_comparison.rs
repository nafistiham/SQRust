use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct FloatingPointComparison;

impl Rule for FloatingPointComparison {
    fn name(&self) -> &'static str {
        "Ambiguous/FloatingPointComparison"
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

            // Look for = != <> operators
            let (is_eq, op_len) = if bytes[i] == b'=' && (i == 0 || (bytes[i - 1] != b'!' && bytes[i - 1] != b'<' && bytes[i - 1] != b'>')) {
                (true, 1)
            } else if i + 1 < len && bytes[i] == b'!' && bytes[i + 1] == b'=' {
                (true, 2)
            } else if i + 1 < len && bytes[i] == b'<' && bytes[i + 1] == b'>' {
                (true, 2)
            } else {
                (false, 1)
            };

            if !is_eq {
                i += 1;
                continue;
            }

            let op_start = i;
            i += op_len;

            // Skip whitespace after operator
            while i < len && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
                i += 1;
            }

            // Check if what follows is a float literal: optional sign, digits, '.', digits
            let float_start = i;
            // Skip optional sign
            if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
                i += 1;
            }
            // Must have at least one digit
            let digit_start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            // Must have a '.'
            if i < len && bytes[i] == b'.' && i > digit_start {
                i += 1;
                // Must have at least one digit after '.'
                let frac_start = i;
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i > frac_start {
                    // Make sure it's not followed by more word chars (like 'e10' making it scientific)
                    let followed_by_word = i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_');
                    if !followed_by_word {
                        // Confirmed float literal
                        let (line, col) = offset_to_line_col(source, op_start);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: "Exact equality comparison with floating-point literal; floating-point values are imprecise — consider using a range check or ROUND()".to_string(),
                            line,
                            col,
                        });
                        continue;
                    }
                }
            }
            // Not a float — reset i to after operator
            i = float_start;
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
