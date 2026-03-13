use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SetExpr, Statement};

pub struct SelectIntoTable;

impl Rule for SelectIntoTable {
    fn name(&self) -> &'static str {
        "Lint/SelectIntoTable"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST may be incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let source = &ctx.source;

        for stmt in &ctx.statements {
            match stmt {
                Statement::Query(query) => {
                    check_query(query, self.name(), source, &mut diags);
                }
                _ => {}
            }
        }

        diags
    }
}

/// Recursively check a Query and all nested queries for SELECT INTO.
fn check_query(query: &Query, rule: &'static str, source: &str, diags: &mut Vec<Diagnostic>) {
    // Check CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, source, diags);
        }
    }

    check_set_expr(&query.body, rule, source, diags);
}

fn check_set_expr(expr: &SetExpr, rule: &'static str, source: &str, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            if sel.into.is_some() {
                let (line, col) = find_keyword_position(source, "SELECT");
                diags.push(Diagnostic {
                    rule,
                    message:
                        "SELECT INTO creates a table implicitly — use CREATE TABLE ... AS SELECT for explicit DDL"
                            .to_string(),
                    line,
                    col,
                });
            }
        }
        SetExpr::Query(inner) => {
            check_query(inner, rule, source, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, rule, source, diags);
            check_set_expr(right, rule, source, diags);
        }
        _ => {}
    }
}

/// Finds the 1-indexed (line, col) of the first occurrence of `keyword` (case-insensitive)
/// in `source` with word boundaries on both sides. Falls back to (1, 1) if not found.
fn find_keyword_position(source: &str, keyword: &str) -> (usize, usize) {
    let kw_upper: String = keyword.to_uppercase();
    let source_upper = source.to_uppercase();
    let kw_len = kw_upper.len();
    let bytes = source_upper.as_bytes();
    let text_len = bytes.len();

    let mut search_from = 0usize;
    while search_from < text_len {
        let Some(rel) = source_upper[search_from..].find(&kw_upper) else {
            break;
        };
        let abs = search_from + rel;

        let before_ok = abs == 0 || {
            let b = bytes[abs - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };
        let after = abs + kw_len;
        let after_ok = after >= text_len || {
            let b = bytes[after];
            !b.is_ascii_alphanumeric() && b != b'_'
        };

        if before_ok && after_ok {
            return offset_to_line_col(source, abs);
        }
        search_from = abs + 1;
    }

    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
