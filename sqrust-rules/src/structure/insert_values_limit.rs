use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{SetExpr, Statement};

/// Flag INSERT INTO ... VALUES (...) statements with more than 50 value row
/// tuples. Large batch inserts can cause memory issues, lock contention, and
/// are hard to debug.
pub struct InsertValuesLimit;

const LIMIT: usize = 50;

impl Rule for InsertValuesLimit {
    fn name(&self) -> &'static str {
        "Structure/InsertValuesLimit"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let source = &ctx.source;
        let source_upper = source.to_uppercase();
        let mut search_from: usize = 0;

        for stmt in &ctx.statements {
            if let Statement::Insert(insert) = stmt {
                // Only inspect the VALUES form — INSERT ... SELECT has a
                // `source` field (Query) with a non-Values body.
                let row_count = match &insert.source {
                    Some(query) => {
                        match query.body.as_ref() {
                            SetExpr::Values(values) => values.rows.len(),
                            // INSERT ... SELECT or other non-VALUES forms.
                            _ => {
                                advance_past_keyword(
                                    source,
                                    &source_upper,
                                    "INSERT",
                                    &mut search_from,
                                );
                                continue;
                            }
                        }
                    }
                    // No source field — shouldn't happen for valid INSERT but
                    // handle gracefully.
                    None => {
                        advance_past_keyword(
                            source,
                            &source_upper,
                            "INSERT",
                            &mut search_from,
                        );
                        continue;
                    }
                };

                if row_count > LIMIT {
                    let (line, col) =
                        find_keyword_position(source, &source_upper, "INSERT", &mut search_from);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: format!(
                            "INSERT with {row_count} value rows exceeds limit of {LIMIT} \
                             — split into smaller batches to avoid lock contention and memory issues"
                        ),
                        line,
                        col,
                    });
                } else {
                    advance_past_keyword(source, &source_upper, "INSERT", &mut search_from);
                }
            }
        }

        diags
    }
}

// ── position helpers ──────────────────────────────────────────────────────────

fn find_keyword_position(
    source: &str,
    source_upper: &str,
    keyword: &str,
    search_from: &mut usize,
) -> (usize, usize) {
    let (line, col, new_from) = find_keyword_inner(source, source_upper, keyword, *search_from);
    *search_from = new_from;
    (line, col)
}

fn advance_past_keyword(
    source: &str,
    source_upper: &str,
    keyword: &str,
    search_from: &mut usize,
) {
    let (_, _, new_from) = find_keyword_inner(source, source_upper, keyword, *search_from);
    *search_from = new_from;
}

fn find_keyword_inner(
    source: &str,
    source_upper: &str,
    keyword: &str,
    start: usize,
) -> (usize, usize, usize) {
    let kw_len = keyword.len();
    let bytes = source_upper.as_bytes();
    let text_len = bytes.len();

    let mut pos = start;
    while pos < text_len {
        let Some(rel) = source_upper[pos..].find(keyword) else {
            break;
        };
        let abs = pos + rel;

        let before_ok = abs == 0
            || {
                let b = bytes[abs - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
        let after = abs + kw_len;
        let after_ok = after >= text_len
            || {
                let b = bytes[after];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

        if before_ok && after_ok {
            let (line, col) = offset_to_line_col(source, abs);
            return (line, col, after);
        }
        pos = abs + 1;
    }

    (1, 1, start)
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
