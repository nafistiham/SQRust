use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct CaseElse;

impl Rule for CaseElse {
    fn name(&self) -> &'static str {
        "Convention/CaseElse"
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
    // Check CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, ctx, diags);
        }
    }

    // Check ORDER BY expressions.
    if let Some(order_by) = &query.order_by {
        for ob_expr in &order_by.exprs {
            check_expr(&ob_expr.expr, ctx, diags);
        }
    }

    // Recurse into the body.
    collect_from_set_expr(&query.body, ctx, diags);
}

fn collect_from_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
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
    // SELECT projection.
    for item in &select.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, ctx, diags);
        }
    }

    // FROM clause — derived subqueries.
    for table_with_joins in &select.from {
        collect_from_table_factor(&table_with_joins.relation, ctx, diags);
        for join in &table_with_joins.joins {
            collect_from_table_factor(&join.relation, ctx, diags);
        }
    }

    // WHERE clause.
    if let Some(selection) = &select.selection {
        check_expr(selection, ctx, diags);
    }

    // HAVING clause.
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

/// Recursively checks an expression tree for CASE expressions missing ELSE.
fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            // Flag if no ELSE clause.
            if else_result.is_none() {
                let (line, col) = find_keyword_pos(&ctx.source, "CASE");
                diags.push(Diagnostic {
                    rule: "Convention/CaseElse",
                    message: "CASE expression has no ELSE clause; unmatched conditions will return NULL"
                        .to_string(),
                    line,
                    col,
                });
            }

            // Recurse into operand.
            if let Some(op) = operand {
                check_expr(op, ctx, diags);
            }

            // Recurse into WHEN conditions.
            for cond in conditions {
                check_expr(cond, ctx, diags);
            }

            // Recurse into THEN results.
            for result in results {
                check_expr(result, ctx, diags);
            }

            // Recurse into ELSE result.
            if let Some(else_e) = else_result {
                check_expr(else_e, ctx, diags);
            }
        }

        // Recurse through other expression types.
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, ctx, diags);
            check_expr(right, ctx, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, ctx, diags);
        }
        Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
            check_expr(inner, ctx, diags);
        }
        Expr::IsDistinctFrom(left, right) | Expr::IsNotDistinctFrom(left, right) => {
            check_expr(left, ctx, diags);
            check_expr(right, ctx, diags);
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
        Expr::Function(f) => {
            if let sqlparser::ast::FunctionArguments::List(arg_list) = &f.args {
                for arg in &arg_list.args {
                    if let sqlparser::ast::FunctionArg::Unnamed(
                        sqlparser::ast::FunctionArgExpr::Expr(e),
                    ) = arg
                    {
                        check_expr(e, ctx, diags);
                    }
                }
            }
        }
        Expr::Cast { expr: inner, .. } => {
            check_expr(inner, ctx, diags);
        }
        Expr::Nested(inner) => {
            check_expr(inner, ctx, diags);
        }
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, ctx, diags);
        }
        _ => {}
    }
}

/// Finds the first occurrence of `keyword` (case-insensitive, word-boundary)
/// in `source` and returns a 1-indexed (line, col) pair.
/// Falls back to (1, 1) if not found.
fn find_keyword_pos(source: &str, keyword: &str) -> (usize, usize) {
    let upper = source.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let kw_len = kw_upper.len();
    let bytes = upper.as_bytes();
    let len = bytes.len();

    let mut pos = 0;
    while pos + kw_len <= len {
        if let Some(rel) = upper[pos..].find(kw_upper.as_str()) {
            let abs = pos + rel;

            let before_ok = abs == 0 || {
                let b = bytes[abs - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            let after = abs + kw_len;
            let after_ok = after >= len || {
                let b = bytes[after];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if before_ok && after_ok {
                return line_col(source, abs);
            }

            pos = abs + 1;
        } else {
            break;
        }
    }

    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
