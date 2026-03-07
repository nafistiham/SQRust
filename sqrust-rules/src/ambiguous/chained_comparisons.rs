use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    BinaryOperator, Expr, GroupByExpr, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
};

pub struct ChainedComparisons;

impl Rule for ChainedComparisons {
    fn name(&self) -> &'static str {
        "Ambiguous/ChainedComparisons"
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
    // Check ORDER BY expressions (they live on Query, not Select).
    if let Some(order_by) = &query.order_by {
        for ob_expr in &order_by.exprs {
            check_expr(&ob_expr.expr, ctx, diags);
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
            // Check JOIN ON conditions.
            use sqlparser::ast::{JoinConstraint, JoinOperator};
            let on_expr = match &join.join_operator {
                JoinOperator::Inner(JoinConstraint::On(e))
                | JoinOperator::LeftOuter(JoinConstraint::On(e))
                | JoinOperator::RightOuter(JoinConstraint::On(e))
                | JoinOperator::FullOuter(JoinConstraint::On(e)) => Some(e),
                _ => None,
            };
            if let Some(e) = on_expr {
                check_expr(e, ctx, diags);
            }
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

    // Check GROUP BY expressions.
    if let GroupByExpr::Expressions(exprs, _) = &select.group_by {
        for e in exprs {
            check_expr(e, ctx, diags);
        }
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

/// Returns true if `op` is one of the six SQL comparison operators.
fn is_comparison_op(op: &BinaryOperator) -> bool {
    matches!(
        op,
        BinaryOperator::Lt
            | BinaryOperator::Gt
            | BinaryOperator::LtEq
            | BinaryOperator::GtEq
            | BinaryOperator::Eq
            | BinaryOperator::NotEq
    )
}

/// Walks `expr`, flagging any `BinaryOp` node that is a comparison whose LEFT
/// child is also a comparison — the classic "chained comparison" pattern.
///
/// Recursion visits both children of every `BinaryOp` so that nested chains
/// (e.g. `a < b < c < d`) are caught at every level.
fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::BinaryOp { left, op, right } => {
            // Check this node for the chained-comparison pattern BEFORE recursing,
            // so the outermost chain is reported first.
            if is_comparison_op(op) {
                if let Expr::BinaryOp { op: inner_op, .. } = left.as_ref() {
                    if is_comparison_op(inner_op) {
                        let (line, col) = find_keyword_position(&ctx.source, "where")
                            .or_else(|| find_keyword_position(&ctx.source, "select"))
                            .unwrap_or((1, 1));
                        diags.push(Diagnostic {
                            rule: "Ambiguous/ChainedComparisons",
                            message:
                                "Chained comparison 'a < b < c' is ambiguous; use 'a < b AND b < c' instead"
                                    .to_string(),
                            line,
                            col,
                        });
                    }
                }
            }
            // Recurse into both children to catch nested chains.
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
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, ctx, diags);
        }
        _ => {}
    }
}

/// Finds the first word-boundary occurrence of `keyword` (case-insensitive) in
/// `source` and returns `Some((line, col))`, or `None` if not found.
fn find_keyword_position(source: &str, keyword: &str) -> Option<(usize, usize)> {
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
                return Some(offset_to_line_col(source, i));
            }
        }
        i += 1;
    }
    None
}

/// Converts a byte offset to 1-indexed (line, col).
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
