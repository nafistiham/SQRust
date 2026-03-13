use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, Function, FunctionArg, FunctionArgExpr, FunctionArguments, OrderByExpr, Query, Select,
    SelectItem, SetExpr, Statement, TableFactor, WindowFrameBound, WindowFrameUnits, WindowType,
};

pub struct WindowFrameAllRows;

impl Rule for WindowFrameAllRows {
    fn name(&self) -> &'static str {
        "Structure/WindowFrameAllRows"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.name(), ctx, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, ctx, diags);
        }
    }

    check_set_expr(&query.body, rule, ctx, diags);

    // Check ORDER BY expressions at query level.
    if let Some(order_by) = &query.order_by {
        for ob in &order_by.exprs {
            check_order_by_expr(ob, rule, ctx, diags);
        }
    }
}

fn check_set_expr(
    expr: &SetExpr,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, rule, ctx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, rule, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, rule, ctx, diags);
            check_set_expr(right, rule, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    for item in &sel.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, rule, ctx, diags);
        }
    }

    if let Some(selection) = &sel.selection {
        check_expr(selection, rule, ctx, diags);
    }

    if let Some(having) = &sel.having {
        check_expr(having, rule, ctx, diags);
    }

    for twj in &sel.from {
        check_table_factor(&twj.relation, rule, ctx, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, rule, ctx, diags);
        }
    }
}

fn check_table_factor(
    tf: &TableFactor,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, rule, ctx, diags);
    }
}

fn check_order_by_expr(
    ob: &OrderByExpr,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    check_expr(&ob.expr, rule, ctx, diags);
}

fn check_expr(
    expr: &Expr,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Function(func) => {
            check_function(func, rule, ctx, diags);
        }
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, rule, ctx, diags);
            check_expr(right, rule, ctx, diags);
        }
        Expr::UnaryOp { expr, .. } => {
            check_expr(expr, rule, ctx, diags);
        }
        Expr::Nested(e) => {
            check_expr(e, rule, ctx, diags);
        }
        Expr::IsNull(e) => {
            check_expr(e, rule, ctx, diags);
        }
        Expr::IsNotNull(e) => {
            check_expr(e, rule, ctx, diags);
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                check_expr(op, rule, ctx, diags);
            }
            for c in conditions {
                check_expr(c, rule, ctx, diags);
            }
            for r in results {
                check_expr(r, rule, ctx, diags);
            }
            if let Some(el) = else_result {
                check_expr(el, rule, ctx, diags);
            }
        }
        Expr::Subquery(q) => {
            check_query(q, rule, ctx, diags);
        }
        Expr::InSubquery { subquery, .. } => {
            check_query(subquery, rule, ctx, diags);
        }
        Expr::Exists { subquery, .. } => {
            check_query(subquery, rule, ctx, diags);
        }
        _ => {}
    }
}

/// Check a Function node: flag window functions with ROWS BETWEEN UNBOUNDED
/// PRECEDING AND UNBOUNDED FOLLOWING but no PARTITION BY.
fn check_function(
    func: &Function,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(WindowType::WindowSpec(spec)) = &func.over {
        if spec.partition_by.is_empty() {
            if let Some(frame) = &spec.window_frame {
                if is_rows_unbounded_all(frame) {
                    let (line, col) = find_over_pos(&ctx.source);
                    diags.push(Diagnostic {
                        rule,
                        message: "Window function with ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING and no PARTITION BY processes the entire table — verify this is intentional".to_string(),
                        line,
                        col,
                    });
                }
            }
        }
    }

    // Recurse into function arguments.
    if let FunctionArguments::List(list) = &func.args {
        for arg in &list.args {
            let fae = match arg {
                FunctionArg::Named { arg, .. }
                | FunctionArg::ExprNamed { arg, .. }
                | FunctionArg::Unnamed(arg) => arg,
            };
            if let FunctionArgExpr::Expr(e) = fae {
                check_expr(e, rule, ctx, diags);
            }
        }
    }
}

/// Returns true when the frame is exactly `ROWS BETWEEN UNBOUNDED PRECEDING
/// AND UNBOUNDED FOLLOWING`.
fn is_rows_unbounded_all(frame: &sqlparser::ast::WindowFrame) -> bool {
    if frame.units != WindowFrameUnits::Rows {
        return false;
    }
    let start_ok = matches!(frame.start_bound, WindowFrameBound::Preceding(None));
    let end_ok = matches!(
        frame.end_bound,
        Some(WindowFrameBound::Following(None))
    );
    start_ok && end_ok
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Scan source for the first `OVER` keyword (case-insensitive, word-boundary)
/// and return its 1-indexed (line, col). Falls back to (1, 1).
fn find_over_pos(source: &str) -> (usize, usize) {
    let keyword = "OVER";
    let upper = source.to_uppercase();
    let kw_len = keyword.len();
    let bytes = upper.as_bytes();
    let len = bytes.len();

    let mut pos = 0;
    while pos + kw_len <= len {
        if let Some(rel) = upper[pos..].find(keyword) {
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
