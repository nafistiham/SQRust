use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct UnionAll;

impl Rule for UnionAll {
    fn name(&self) -> &'static str {
        "Structure/UnionAll"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Skip non-code bytes (strings, comments, quoted identifiers).
            if !skip_map.is_code(i) {
                i += 1;
                continue;
            }

            // Look for a word boundary start.
            if !is_word_char(bytes[i]) || (i > 0 && is_word_char(bytes[i - 1])) {
                i += 1;
                continue;
            }

            // Identify the end of this word token.
            let word_start = i;
            let mut j = i;
            while j < len && is_word_char(bytes[j]) {
                j += 1;
            }
            let word_end = j;

            // Ensure the entire word is in code (no skip inside the word).
            let all_code = (word_start..word_end).all(|k| skip_map.is_code(k));

            if all_code {
                let word_bytes = &bytes[word_start..word_end];

                // Case-insensitive match for "UNION".
                let is_union = word_bytes.len() == 5
                    && b"UNION"
                        .iter()
                        .zip(word_bytes.iter())
                        .all(|(a, b)| a.eq_ignore_ascii_case(b));

                if is_union {
                    // Skip whitespace (including newlines) after UNION.
                    let mut k = word_end;
                    while k < len && (bytes[k] == b' ' || bytes[k] == b'\t' || bytes[k] == b'\n' || bytes[k] == b'\r') {
                        k += 1;
                    }

                    // Read the next word.
                    let next_word_start = k;
                    while k < len && is_word_char(bytes[k]) {
                        k += 1;
                    }
                    let next_word_end = k;

                    let next_word = &bytes[next_word_start..next_word_end];

                    let is_all = next_word.len() == 3
                        && b"ALL"
                            .iter()
                            .zip(next_word.iter())
                            .all(|(a, b)| a.eq_ignore_ascii_case(b));

                    let is_distinct = next_word.len() == 8
                        && b"DISTINCT"
                            .iter()
                            .zip(next_word.iter())
                            .all(|(a, b)| a.eq_ignore_ascii_case(b));

                    if !is_all && !is_distinct {
                        let (line, col) = line_col(source, word_start);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: "Prefer UNION ALL or UNION DISTINCT over bare UNION to make intent explicit".to_string(),
                            line,
                            col,
                        });
                    }
                }
            }

            i = word_end;
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
