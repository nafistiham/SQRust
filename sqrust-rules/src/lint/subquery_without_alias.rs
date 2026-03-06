use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SetExpr, Statement, TableFactor};

pub struct SubqueryWithoutAlias;

impl Rule for SubqueryWithoutAlias {
    fn name(&self) -> &'static str {
        "Lint/SubqueryWithoutAlias"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &ctx.source, &mut diags);
            }
        }
        diags
    }
}

fn check_query(query: &Query, source: &str, diags: &mut Vec<Diagnostic>) {
    // Check CTE bodies.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, diags);
        }
    }
    check_set_expr(&query.body, source, diags);
}

fn check_set_expr(expr: &SetExpr, source: &str, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            for table in &sel.from {
                check_table_factor(&table.relation, source, diags);
                for join in &table.joins {
                    check_table_factor(&join.relation, source, diags);
                }
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, diags);
            check_set_expr(right, source, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, source, diags);
        }
        _ => {}
    }
}

fn check_table_factor(tf: &TableFactor, source: &str, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived {
        subquery, alias, ..
    } = tf
    {
        if alias.is_none() {
            // Find the opening `(` for the subquery's position in source.
            let (line, col) = find_subquery_position(source, subquery);
            diags.push(Diagnostic {
                rule: "Lint/SubqueryWithoutAlias",
                message: "Derived table (subquery in FROM) has no alias; add an alias for portability".to_string(),
                line,
                col,
            });
        }
        // Always recurse into the subquery to catch nested unaliased derived tables.
        check_query(subquery, source, diags);
    }
}

/// Finds the `(SELECT` (or just `(`) that opens `subquery` inside `source`.
/// Returns a 1-indexed (line, col) pair; falls back to (1, 1) if not found.
fn find_subquery_position(source: &str, _subquery: &Query) -> (usize, usize) {
    // Locate the first `(SELECT` in source (case-insensitive).
    let source_upper = source.to_uppercase();
    let needle = "(SELECT";

    // Walk all occurrences and return the first one (leftmost).
    if let Some(pos) = source_upper.find(needle) {
        return offset_to_line_col(source, pos);
    }

    // Fallback: find the first bare `(`.
    if let Some(pos) = source.find('(') {
        return offset_to_line_col(source, pos);
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
