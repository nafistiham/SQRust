use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct DeeplyNestedCase {
    pub max_depth: usize,
}

impl Default for DeeplyNestedCase {
    fn default() -> Self {
        Self { max_depth: 3 }
    }
}

impl Rule for DeeplyNestedCase {
    fn name(&self) -> &'static str {
        "Structure/DeeplyNestedCase"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        if source.is_empty() {
            return Vec::new();
        }

        let skip = SkipMap::build(source);
        let bytes = source.as_bytes();
        let len = bytes.len();

        let mut depth: usize = 0;
        let mut max_depth: usize = 0;
        // byte offset where max_depth was first exceeded
        let mut violation_offset: Option<usize> = None;

        let mut i = 0usize;
        while i < len {
            // Only examine bytes that are real SQL code (not in strings/comments).
            if !skip.is_code(i) {
                i += 1;
                continue;
            }

            // Try to match "CASE" keyword (case-insensitive, word boundary).
            if i + 4 <= len {
                let slice = &bytes[i..i + 4];
                let matches_case = slice[0].to_ascii_uppercase() == b'C'
                    && slice[1].to_ascii_uppercase() == b'A'
                    && slice[2].to_ascii_uppercase() == b'S'
                    && slice[3].to_ascii_uppercase() == b'E';

                if matches_case {
                    let before_ok =
                        i == 0 || !is_word_char(bytes[i - 1]);
                    let after = i + 4;
                    let after_ok = after >= len || !is_word_char(bytes[after]);

                    if before_ok && after_ok {
                        depth += 1;
                        if depth > max_depth {
                            max_depth = depth;
                            if depth > self.max_depth && violation_offset.is_none() {
                                violation_offset = Some(i);
                            }
                        }
                        i += 4;
                        continue;
                    }
                }
            }

            // Try to match "END" keyword (case-insensitive, word boundary).
            if i + 3 <= len {
                let slice = &bytes[i..i + 3];
                let matches_end = slice[0].to_ascii_uppercase() == b'E'
                    && slice[1].to_ascii_uppercase() == b'N'
                    && slice[2].to_ascii_uppercase() == b'D';

                if matches_end {
                    let before_ok =
                        i == 0 || !is_word_char(bytes[i - 1]);
                    let after = i + 3;
                    let after_ok = after >= len || !is_word_char(bytes[after]);

                    if before_ok && after_ok && depth > 0 {
                        depth -= 1;
                        i += 3;
                        continue;
                    }
                }
            }

            i += 1;
        }

        if let Some(offset) = violation_offset {
            let (line, col) = offset_to_line_col(source, offset);
            vec![Diagnostic {
                rule: self.name(),
                message: format!(
                    "CASE expression nested {} levels deep (max {}); \
                     refactor into separate CTEs or helper columns",
                    max_depth, self.max_depth
                ),
                line,
                col,
            }]
        } else {
            Vec::new()
        }
    }
}

/// Converts a byte offset into `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
