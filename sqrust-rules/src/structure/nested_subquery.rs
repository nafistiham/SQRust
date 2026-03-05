use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct NestedSubquery {
    /// Maximum number of subquery nesting levels allowed.
    /// Queries with more `(SELECT` patterns than this are flagged.
    pub max_depth: usize,
}

impl Default for NestedSubquery {
    fn default() -> Self {
        NestedSubquery { max_depth: 2 }
    }
}

impl Rule for NestedSubquery {
    fn name(&self) -> &'static str {
        "Structure/NestedSubquery"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);

        // Count occurrences of `(` followed by optional whitespace followed by
        // the keyword SELECT (word-boundary on both sides).
        //
        // Each such pattern represents one level of subquery nesting.
        // We record the byte offset of the SELECT keyword at which we first
        // exceed max_depth so we can provide an accurate line/col.
        let mut depth: usize = 0;
        let mut first_excess_offset: Option<usize> = None;

        let mut i = 0;
        while i < len {
            // Skip bytes that are inside strings, comments, or quoted identifiers.
            if !skip_map.is_code(i) {
                i += 1;
                continue;
            }

            let b = bytes[i];

            // Look for `(` in code.
            if b == b'(' {
                // Scan forward past optional whitespace to find SELECT.
                let mut j = i + 1;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                    j += 1;
                }

                // j now points at the first non-whitespace byte after `(`.
                // Check whether it starts the keyword SELECT (case-insensitive,
                // word-boundary after).
                if j + 6 <= len {
                    let candidate = &bytes[j..j + 6];
                    let is_select = b"SELECT"
                        .iter()
                        .zip(candidate.iter())
                        .all(|(a, b)| a.eq_ignore_ascii_case(b));

                    // Word boundary after SELECT: next byte must not be alphanumeric/_
                    let boundary_after = j + 6 >= len || {
                        let nb = bytes[j + 6];
                        !is_word_char(nb)
                    };

                    // All bytes of SELECT must be real code (not inside a skip region).
                    let all_code = (j..j + 6).all(|k| skip_map.is_code(k));

                    if is_select && boundary_after && all_code {
                        depth += 1;
                        if depth > self.max_depth && first_excess_offset.is_none() {
                            first_excess_offset = Some(j);
                        }
                    }
                }

                i += 1;
                continue;
            }

            i += 1;
        }

        if depth > self.max_depth {
            let offset = first_excess_offset.unwrap_or(0);
            let (line, col) = line_col(source, offset);
            vec![Diagnostic {
                rule: self.name(),
                message: format!(
                    "Subquery nesting depth {depth} exceeds maximum of {max}; consider using CTEs",
                    depth = depth,
                    max = self.max_depth,
                ),
                line,
                col,
            }]
        } else {
            Vec::new()
        }
    }
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
