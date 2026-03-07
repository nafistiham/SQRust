use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct ClauseOnNewLine;

/// Clause keywords that should appear at the start of a line (modulo leading whitespace)
/// when the query spans multiple lines.
const CLAUSES: &[&str] = &[
    "FROM", "WHERE", "GROUP BY", "HAVING", "ORDER BY", "LIMIT", "OFFSET",
    "UNION", "INTERSECT", "EXCEPT", "JOIN", "INNER JOIN", "LEFT JOIN",
    "RIGHT JOIN", "FULL JOIN", "CROSS JOIN",
];

impl Rule for ClauseOnNewLine {
    fn name(&self) -> &'static str {
        "Layout/ClauseOnNewLine"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let src = &ctx.source;
        // Only check multi-line queries.
        if !src.contains('\n') {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let lines: Vec<&str> = src.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            // Determine the leading whitespace length.
            let leading = line.len() - trimmed.len();

            for clause in CLAUSES {
                // Does this line contain the clause keyword NOT at the start?
                // "Not at the start" means there is non-whitespace content before it.
                let upper = trimmed.to_ascii_uppercase();
                if let Some(pos) = find_word(upper.as_bytes(), clause.as_bytes()) {
                    if pos > 0 {
                        // There is non-whitespace content before the clause on this line.
                        // Compute actual col in original line (1-indexed).
                        let before_in_line = &line[..leading + pos];
                        let line_col_col = before_in_line.len() + 1;
                        diags.push(Diagnostic {
                            rule: "Layout/ClauseOnNewLine",
                            message: format!(
                                "Clause '{}' should start on its own line",
                                clause
                            ),
                            line: line_idx + 1,
                            col: line_col_col,
                        });
                        // Only flag the first clause on each line.
                        break;
                    }
                }
            }
        }

        diags
    }
}

/// Find the byte offset of `needle` (uppercased) in `haystack` (already uppercased),
/// requiring word boundaries. Returns `None` if not found.
fn find_word(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let n_len = needle.len();
    let h_len = haystack.len();
    if n_len > h_len {
        return None;
    }
    let mut i = 0;
    while i + n_len <= h_len {
        if &haystack[i..i + n_len] == needle {
            let before_ok = i == 0 || !is_word(haystack[i - 1]);
            let after = i + n_len;
            let after_ok = after >= h_len || !is_word(haystack[after]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_word(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}
