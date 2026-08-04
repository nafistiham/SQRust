use sqrust_core::{Diagnostic, FileContext, Rule};

use crate::capitalisation::SkipMap;

pub struct ClauseOnNewLine;

/// Clause keywords that should appear at the start of a line (modulo leading
/// whitespace) when the query spans multiple lines.
///
/// Compound clauses come first so `LEFT JOIN` is preferred over the bare
/// `JOIN` inside it — otherwise a correctly formatted `LEFT JOIN` at column 0
/// is reported because `JOIN` sits at offset 5.
const CLAUSES: &[&str] = &[
    "CROSS JOIN", "INNER JOIN", "RIGHT JOIN", "LEFT JOIN", "FULL JOIN",
    "GROUP BY", "ORDER BY", "INTERSECT", "UNION", "EXCEPT", "HAVING",
    "OFFSET", "WHERE", "LIMIT", "FROM", "JOIN",
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

        let bytes = src.as_bytes();
        let skip = SkipMap::build(src);

        // Paren nesting depth immediately before each byte, counting only
        // real code. Clause keywords nested inside parentheses belong to a
        // window spec (`OVER (... ORDER BY ...)`), a subquery, or an
        // expression list — none of which this rule governs.
        let mut depth_at = vec![0u32; bytes.len() + 1];
        let mut depth: u32 = 0;
        for i in 0..bytes.len() {
            depth_at[i] = depth;
            if skip.is_code(i) {
                if bytes[i] == b'(' {
                    depth += 1;
                } else if bytes[i] == b')' {
                    depth = depth.saturating_sub(1);
                }
            }
        }
        depth_at[bytes.len()] = depth;

        let mut diags = Vec::new();
        let mut line_start = 0usize;
        let mut line_idx = 0usize;

        loop {
            let line_end = src[line_start..]
                .find('\n')
                .map(|p| line_start + p)
                .unwrap_or(bytes.len());
            let raw = &src[line_start..line_end];
            let line = raw.strip_suffix('\r').unwrap_or(raw);

            let trimmed = line.trim_start();
            let leading = line.len() - trimmed.len();

            // A comment line is prose, not SQL layout.
            if !trimmed.starts_with("--") {
                let upper = trimmed.to_ascii_uppercase();
                let ub = upper.as_bytes();

                // Scan left to right, consuming the longest clause that
                // matches at each position. A line may legitimately begin with
                // a clause; it is any *subsequent* clause on the same line
                // that is the violation. Consuming the whole match is what
                // stops the `JOIN` inside `LEFT JOIN` being reported.
                let mut i = 0usize;
                while i < ub.len() {
                    let mut matched: Option<&str> = None;
                    for clause in CLAUSES {
                        let cb = clause.as_bytes();
                        if i + cb.len() <= ub.len()
                            && &ub[i..i + cb.len()] == cb
                            && (i == 0 || !is_word(ub[i - 1]))
                            && (i + cb.len() >= ub.len() || !is_word(ub[i + cb.len()]))
                            && matched.is_none_or(|m: &str| clause.len() > m.len())
                        {
                            matched = Some(clause);
                        }
                    }

                    let Some(clause) = matched else {
                        i += 1;
                        continue;
                    };

                    let abs = line_start + leading + i;
                    // `i > 0` means real content precedes the clause.
                    // `is_code` rejects keywords inside strings and comments.
                    // `depth_at == 0` rejects window specs and subqueries.
                    if i > 0 && skip.is_code(abs) && depth_at[abs] == 0 {
                        diags.push(Diagnostic {
                            rule: "Layout/ClauseOnNewLine",
                            message: format!("Clause '{}' should start on its own line", clause),
                            line: line_idx + 1,
                            col: leading + i + 1,
                        });
                        // Only flag the first offending clause per line.
                        break;
                    }
                    i += clause.len();
                }
            }

            if line_end >= bytes.len() {
                break;
            }
            line_start = line_end + 1;
            line_idx += 1;
        }

        diags
    }
}

fn is_word(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}
