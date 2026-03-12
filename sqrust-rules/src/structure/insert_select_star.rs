use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SelectItem, SetExpr, Statement};

pub struct InsertSelectStar;

impl Rule for InsertSelectStar {
    fn name(&self) -> &'static str {
        "Structure/InsertSelectStar"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST may be incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let source = &ctx.source;
        let source_upper = source.to_uppercase();
        let mut search_from = 0usize;

        for stmt in &ctx.statements {
            if let Statement::Insert(insert) = stmt {
                if let Some(src_query) = &insert.source {
                    if query_top_level_has_wildcard(src_query) {
                        let (line, col) = find_keyword_position(
                            source,
                            &source_upper,
                            "INSERT",
                            &mut search_from,
                        );
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message:
                                "INSERT ... SELECT * is fragile — the column order of the source \
                                 table may change silently. Specify explicit columns instead"
                                    .to_string(),
                            line,
                            col,
                        });
                    } else {
                        // No wildcard — still advance past this INSERT keyword.
                        advance_past_keyword(source, &source_upper, "INSERT", &mut search_from);
                    }
                } else {
                    // VALUES form or no source — advance past INSERT.
                    advance_past_keyword(source, &source_upper, "INSERT", &mut search_from);
                }
            }
        }

        diags
    }
}

/// Returns true if the top-level SELECT projection of a query contains a
/// wildcard (`*` or `table.*`). Does NOT descend into subqueries within the
/// projection list — only the outermost SELECT items are checked.
fn query_top_level_has_wildcard(query: &Query) -> bool {
    match query.body.as_ref() {
        SetExpr::Select(sel) => sel.projection.iter().any(|item| {
            matches!(
                item,
                SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
            )
        }),
        // A parenthesised sub-query body — recurse one level.
        SetExpr::Query(inner) => query_top_level_has_wildcard(inner),
        // UNION / INTERSECT / EXCEPT — check the left branch (which is the
        // immediately visible SELECT for the INSERT).
        SetExpr::SetOperation { left, .. } => match left.as_ref() {
            SetExpr::Select(sel) => sel.projection.iter().any(|item| {
                matches!(
                    item,
                    SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
                )
            }),
            SetExpr::Query(inner) => query_top_level_has_wildcard(inner),
            _ => false,
        },
        _ => false,
    }
}

// ── position helpers (shared pattern with other rules) ────────────────────────

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
