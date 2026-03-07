use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, OrderBy, Query, SetExpr, Statement, TableFactor};

pub struct SubqueryInOrderBy;

impl Rule for SubqueryInOrderBy {
    fn name(&self) -> &'static str {
        "Ambiguous/SubqueryInOrderBy"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            collect_from_statement(stmt, ctx, &mut diags);
        }
        diags
    }
}

fn collect_from_statement(stmt: &Statement, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Statement::Query(query) = stmt {
        collect_from_query(query, ctx, diags);
    }
}

fn collect_from_query(query: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Recurse into CTEs first.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, ctx, diags);
        }
    }

    // Check ORDER BY on this query.
    if let Some(order_by) = &query.order_by {
        check_order_by(order_by, ctx, diags);
    }

    // Recurse into body (subqueries in FROM, UNION arms, etc.).
    collect_from_set_expr(&query.body, ctx, diags);
}

fn check_order_by(order_by: &OrderBy, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    for order_expr in &order_by.exprs {
        if contains_subquery(&order_expr.expr) {
            let (line, col) = find_order_by_position(&ctx.source).unwrap_or((1, 1));
            diags.push(Diagnostic {
                rule: "Ambiguous/SubqueryInOrderBy",
                message: "Subquery in ORDER BY is ambiguous and potentially expensive".to_string(),
                line,
                col,
            });
        }
    }
}

fn collect_from_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(select) => {
            // Recurse into derived tables in FROM.
            for twj in &select.from {
                collect_from_table_factor(&twj.relation, ctx, diags);
                for join in &twj.joins {
                    collect_from_table_factor(&join.relation, ctx, diags);
                }
            }
        }
        SetExpr::Query(inner) => {
            collect_from_query(inner, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            collect_from_set_expr(left, ctx, diags);
            collect_from_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}

fn collect_from_table_factor(
    factor: &TableFactor,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        collect_from_query(subquery, ctx, diags);
    }
}

/// Returns `true` if `expr` is or contains a subquery (`Subquery`, `InSubquery`,
/// or `Exists`). Recurses into `BinaryOp`, `UnaryOp`, and `Nested` to catch
/// subqueries embedded inside larger expressions.
fn contains_subquery(expr: &Expr) -> bool {
    match expr {
        Expr::Subquery(_) | Expr::Exists { .. } | Expr::InSubquery { .. } => true,
        Expr::BinaryOp { left, right, .. } => {
            contains_subquery(left) || contains_subquery(right)
        }
        Expr::UnaryOp { expr: inner, .. } => contains_subquery(inner),
        Expr::Nested(inner) => contains_subquery(inner),
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            operand.as_deref().is_some_and(contains_subquery)
                || conditions.iter().any(contains_subquery)
                || results.iter().any(contains_subquery)
                || else_result.as_deref().is_some_and(contains_subquery)
        }
        _ => false,
    }
}

/// Finds the first occurrence of `ORDER BY` (case-insensitive, outside string
/// literals) and returns `Some((line, col))`. Returns `None` if not found.
fn find_order_by_position(source: &str) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    let upper = source.to_ascii_uppercase();
    let upper_bytes = upper.as_bytes();
    // "ORDER BY" is exactly 8 bytes.
    let needle = b"ORDER BY";
    let mut in_string = false;
    let mut i = 0;

    while i < bytes.len() {
        // Track single-quoted SQL string literals.
        if !in_string && bytes[i] == b'\'' {
            in_string = true;
            i += 1;
            continue;
        }
        if in_string {
            if bytes[i] == b'\'' {
                // Escaped quote inside string: two consecutive single-quotes.
                if i + 1 < bytes.len() && bytes[i + 1] == b'\'' {
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }

        // Try to match "ORDER BY" at a word boundary.
        if i + needle.len() <= upper_bytes.len()
            && &upper_bytes[i..i + needle.len()] == needle
        {
            let before_ok =
                i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let after = i + needle.len();
            let after_ok = after >= bytes.len()
                || (!bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_');
            if before_ok && after_ok {
                return Some(offset_to_line_col(source, i));
            }
        }

        i += 1;
    }

    None
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
