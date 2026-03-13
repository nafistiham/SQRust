use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
};

pub struct InSubqueryMultiColumn;

impl Rule for InSubqueryMultiColumn {
    fn name(&self) -> &'static str {
        "Ambiguous/InSubqueryMultiColumn"
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
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, ctx, diags);
        }
    }
    collect_from_set_expr(&query.body, ctx, diags);
}

fn collect_from_set_expr(set_expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match set_expr {
        SetExpr::Select(select) => {
            collect_from_select(select, ctx, diags);
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

fn collect_from_select(select: &Select, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Check SELECT projection expressions.
    for item in &select.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, ctx, diags);
        }
    }

    // Check FROM subqueries.
    for twj in &select.from {
        collect_from_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            collect_from_table_factor(&join.relation, ctx, diags);
        }
    }

    // Check WHERE.
    if let Some(selection) = &select.selection {
        check_expr(selection, ctx, diags);
    }

    // Check HAVING.
    if let Some(having) = &select.having {
        check_expr(having, ctx, diags);
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

/// Returns the number of non-wildcard columns in the top-level SELECT of a query body.
/// Returns None if the body is not a simple SELECT (e.g. a set operation).
fn count_non_wildcard_columns(body: &SetExpr) -> Option<usize> {
    match body {
        SetExpr::Select(select) => {
            let count = select
                .projection
                .iter()
                .filter(|item| {
                    !matches!(
                        item,
                        SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
                    )
                })
                .count();
            Some(count)
        }
        SetExpr::SetOperation { left, .. } => {
            // For UNION/INTERSECT/EXCEPT, check the left branch column count.
            count_non_wildcard_columns(left)
        }
        SetExpr::Query(inner) => count_non_wildcard_columns(&inner.body),
        _ => None,
    }
}

/// Recursively checks expressions for IN subqueries with more than one column.
fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::InSubquery { expr, subquery, .. } => {
            // Row-constructor pattern: `(a, b) IN (SELECT a, b FROM s)`.
            // The expr side is a Tuple — this is a legitimate multi-column row comparison
            // supported by some databases. Skip it.
            let is_row_constructor = matches!(expr.as_ref(), Expr::Tuple(_));

            if !is_row_constructor {
                // Check how many columns the subquery selects.
                if let Some(col_count) = count_non_wildcard_columns(&subquery.body) {
                    if col_count > 1 {
                        let (line, col) = find_in_subquery_pos(&ctx.source);
                        diags.push(Diagnostic {
                            rule: "Ambiguous/InSubqueryMultiColumn",
                            message: format!(
                                "IN subquery selects {} columns — an IN subquery must return exactly one column for portability",
                                col_count
                            ),
                            line,
                            col,
                        });
                    }
                }
            }
            // Recurse into the subquery itself for nested occurrences.
            collect_from_query(subquery, ctx, diags);
        }
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, ctx, diags);
            check_expr(right, ctx, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, ctx, diags);
        }
        Expr::Nested(inner) => {
            check_expr(inner, ctx, diags);
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                check_expr(op, ctx, diags);
            }
            for cond in conditions {
                check_expr(cond, ctx, diags);
            }
            for result in results {
                check_expr(result, ctx, diags);
            }
            if let Some(else_e) = else_result {
                check_expr(else_e, ctx, diags);
            }
        }
        Expr::InList { expr: inner, list, .. } => {
            check_expr(inner, ctx, diags);
            for e in list {
                check_expr(e, ctx, diags);
            }
        }
        Expr::Between {
            expr: inner,
            low,
            high,
            ..
        } => {
            check_expr(inner, ctx, diags);
            check_expr(low, ctx, diags);
            check_expr(high, ctx, diags);
        }
        Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
            check_expr(inner, ctx, diags);
        }
        Expr::Subquery(q) | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, ctx, diags);
        }
        _ => {}
    }
}

/// Finds the position of "IN (" or "IN(" followed by "SELECT" in the source.
/// Falls back to finding plain "IN" keyword position. Falls back to (1, 1).
fn find_in_subquery_pos(source: &str) -> (usize, usize) {
    let upper = source.to_uppercase();
    let bytes = upper.as_bytes();
    let len = bytes.len();

    let mut i = 0;
    while i < len {
        // Look for word-boundary "IN"
        if i + 2 <= len && bytes[i..i + 2] == *b"IN" {
            let before_ok =
                i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let after2 = i + 2;
            let after_ok = after2 >= len
                || (!bytes[after2].is_ascii_alphanumeric() && bytes[after2] != b'_');
            if before_ok && after_ok {
                return offset_to_line_col(source, i);
            }
        }
        i += 1;
    }
    (1, 1)
}

/// Converts a byte offset to 1-indexed (line, col).
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
