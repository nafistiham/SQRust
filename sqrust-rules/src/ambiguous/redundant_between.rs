use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor, Value};

pub struct RedundantBetween;

impl Rule for RedundantBetween {
    fn name(&self) -> &'static str {
        "Ambiguous/RedundantBetween"
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
    // SELECT projection.
    for item in &select.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, ctx, diags);
        }
    }

    // FROM subqueries.
    for twj in &select.from {
        collect_from_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
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

/// Returns `true` when the expression is a literal type or plain identifier
/// that is safe to compare for redundancy without false positives.
fn is_safe_bound(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Value(Value::Number(_, _))
            | Expr::Value(Value::SingleQuotedString(_))
            | Expr::Value(Value::Boolean(_))
            | Expr::Identifier(_)
    )
}

fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Between {
            expr: inner,
            negated,
            low,
            high,
        } => {
            // Recurse into the subject and bounds.
            check_expr(inner, ctx, diags);
            check_expr(low, ctx, diags);
            check_expr(high, ctx, diags);

            // Only flag when both bounds are safe-to-compare literals or identifiers
            // and they are equal.
            if is_safe_bound(low) && is_safe_bound(high) && low == high {
                let keyword = if *negated { "NOT BETWEEN" } else { "BETWEEN" };
                let (line, col) = find_keyword_position(&ctx.source, keyword);
                let message = if *negated {
                    "NOT BETWEEN with identical bounds; use != instead".to_string()
                } else {
                    "BETWEEN with identical bounds; use = instead".to_string()
                };
                diags.push(Diagnostic {
                    rule: "Ambiguous/RedundantBetween",
                    message,
                    line,
                    col,
                });
            }
        }

        // Recurse through other expression types to find nested BETWEEN nodes.
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
        Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
            check_expr(inner, ctx, diags);
        }
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, ctx, diags);
        }
        _ => {}
    }
}

/// Finds the first word-boundary occurrence of `keyword` (case-insensitive) in
/// `source` and returns a 1-indexed (line, col). Falls back to (1, 1).
fn find_keyword_position(source: &str, keyword: &str) -> (usize, usize) {
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

/// Converts a byte offset to 1-indexed (line, col).
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
