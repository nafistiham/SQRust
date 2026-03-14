use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, FunctionArguments, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
};

pub struct DistinctWithWindowFunction;

impl Rule for DistinctWithWindowFunction {
    fn name(&self) -> &'static str {
        "Ambiguous/DistinctWithWindowFunction"
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
    // Only flag if this SELECT has DISTINCT.
    if select.distinct.is_some() {
        // Check if any projection item contains a window function.
        let has_window_fn = select.projection.iter().any(|item| {
            let expr = match item {
                SelectItem::UnnamedExpr(e) => e,
                SelectItem::ExprWithAlias { expr: e, .. } => e,
                _ => return false,
            };
            expr_contains_window_fn(expr)
        });

        if has_window_fn {
            let (line, col) = find_keyword_pos(&ctx.source, "SELECT");
            diags.push(Diagnostic {
                rule: "Ambiguous/DistinctWithWindowFunction",
                message: "DISTINCT with window functions may produce unexpected results — window functions run before DISTINCT is applied".to_string(),
                line,
                col,
            });
        }
    }

    // Always recurse into FROM subqueries.
    for twj in &select.from {
        collect_from_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            collect_from_table_factor(&join.relation, ctx, diags);
        }
    }

    // Recurse into WHERE subqueries.
    if let Some(selection) = &select.selection {
        collect_subqueries_from_expr(selection, ctx, diags);
    }

    // Recurse into HAVING subqueries.
    if let Some(having) = &select.having {
        collect_subqueries_from_expr(having, ctx, diags);
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

/// Returns `true` if `expr` or any sub-expression is a window function call
/// (i.e., a `Function` node with `over: Some(_)`).
fn expr_contains_window_fn(expr: &Expr) -> bool {
    match expr {
        Expr::Function(func) => {
            if func.over.is_some() {
                return true;
            }
            // Recurse into function arguments.
            if let FunctionArguments::List(arg_list) = &func.args {
                use sqlparser::ast::{FunctionArg, FunctionArgExpr};
                for arg in &arg_list.args {
                    let expr_arg = match arg {
                        FunctionArg::Named { arg, .. }
                        | FunctionArg::ExprNamed { arg, .. }
                        | FunctionArg::Unnamed(arg) => arg,
                    };
                    if let FunctionArgExpr::Expr(e) = expr_arg {
                        if expr_contains_window_fn(e) {
                            return true;
                        }
                    }
                }
            }
            false
        }
        Expr::BinaryOp { left, right, .. } => {
            expr_contains_window_fn(left) || expr_contains_window_fn(right)
        }
        Expr::UnaryOp { expr: inner, .. } => expr_contains_window_fn(inner),
        Expr::Nested(inner) => expr_contains_window_fn(inner),
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            operand.as_deref().map_or(false, expr_contains_window_fn)
                || conditions.iter().any(expr_contains_window_fn)
                || results.iter().any(expr_contains_window_fn)
                || else_result
                    .as_deref()
                    .map_or(false, expr_contains_window_fn)
        }
        _ => false,
    }
}

/// Recurse into subqueries nested inside WHERE / HAVING expressions.
fn collect_subqueries_from_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, ctx, diags);
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_subqueries_from_expr(left, ctx, diags);
            collect_subqueries_from_expr(right, ctx, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            collect_subqueries_from_expr(inner, ctx, diags);
        }
        Expr::Nested(inner) => {
            collect_subqueries_from_expr(inner, ctx, diags);
        }
        _ => {}
    }
}

/// Finds the first occurrence of `keyword` (case-insensitive, word-boundary)
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
