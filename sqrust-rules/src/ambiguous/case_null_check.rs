use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    BinaryOperator, Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor, Value,
};

pub struct CaseNullCheck;

impl Rule for CaseNullCheck {
    fn name(&self) -> &'static str {
        "Ambiguous/CaseNullCheck"
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
    // Check SELECT projection.
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

/// Returns true if the expression is the SQL NULL literal.
fn is_null_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Value(Value::Null))
}

/// Returns true if the operator is equality or inequality.
fn is_eq_or_neq(op: &BinaryOperator) -> bool {
    matches!(op, BinaryOperator::Eq | BinaryOperator::NotEq)
}

/// Checks an expression recursively. Flags CASE WHEN conditions that
/// compare with = NULL or <> NULL.
fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            // For `CASE col WHEN NULL THEN ...` (operand form), conditions hold
            // the comparison values. A condition of Value::Null means comparing
            // operand = NULL, which is always unknown.
            if operand.is_some() {
                for cond in conditions {
                    if is_null_literal(cond) {
                        let (line, col) = find_keyword_pos(&ctx.source, "WHEN");
                        diags.push(Diagnostic {
                            rule: "Ambiguous/CaseNullCheck",
                            message: "CASE WHEN col = NULL will never match — use IS NULL or IS NOT NULL instead".to_string(),
                            line,
                            col,
                        });
                    }
                    // Still recurse into the condition expression for nested cases.
                    check_expr(cond, ctx, diags);
                }
            } else {
                // `CASE WHEN <condition> THEN ...` form.
                for cond in conditions {
                    // Check if this condition is col = NULL or col <> NULL.
                    if let Expr::BinaryOp { left, op, right } = cond {
                        if is_eq_or_neq(op)
                            && (is_null_literal(left) || is_null_literal(right))
                        {
                            let (line, col) = find_keyword_pos(&ctx.source, "WHEN");
                            diags.push(Diagnostic {
                                rule: "Ambiguous/CaseNullCheck",
                                message: "CASE WHEN col = NULL will never match — use IS NULL or IS NOT NULL instead".to_string(),
                                line,
                                col,
                            });
                        }
                    }
                    // Recurse to catch nested CASE expressions inside conditions.
                    check_expr(cond, ctx, diags);
                }
            }

            // Recurse into result expressions for nested CASE.
            for result in results {
                check_expr(result, ctx, diags);
            }
            if let Some(else_e) = else_result {
                check_expr(else_e, ctx, diags);
            }
            if let Some(op) = operand {
                check_expr(op, ctx, diags);
            }
        }
        Expr::BinaryOp { left, op, right } => {
            check_expr(left, ctx, diags);
            check_expr(right, ctx, diags);
            let _ = op; // comparison outside CASE is handled by other rules
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, ctx, diags);
        }
        Expr::Nested(inner) => {
            check_expr(inner, ctx, diags);
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
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, ctx, diags);
        }
        _ => {}
    }
}

/// Finds the first occurrence of a keyword (case-insensitive, word-boundary)
/// in `source` and returns a 1-indexed (line, col). Falls back to (1, 1).
fn find_keyword_pos(source: &str, keyword: &str) -> (usize, usize) {
    let upper = source.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let bytes = upper.as_bytes();
    let kw_bytes = kw_upper.as_bytes();
    let kw_len = kw_bytes.len();

    let mut i = 0;
    while i + kw_len <= bytes.len() {
        if bytes[i..i + kw_len] == *kw_bytes {
            let before_ok =
                i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let after = i + kw_len;
            let after_ok = after >= bytes.len()
                || (!bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_');
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
