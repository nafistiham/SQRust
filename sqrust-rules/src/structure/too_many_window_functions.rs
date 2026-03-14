use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Function, FunctionArg, FunctionArgExpr, FunctionArguments, Query, Select, SelectItem, SetExpr, Statement, TableFactor, WindowType};

pub struct TooManyWindowFunctions {
    /// Maximum number of window functions allowed in a single SELECT.
    /// SELECTs with more window functions than this are flagged.
    pub max: usize,
}

impl Default for TooManyWindowFunctions {
    fn default() -> Self {
        TooManyWindowFunctions { max: 5 }
    }
}

impl Rule for TooManyWindowFunctions {
    fn name(&self) -> &'static str {
        "Structure/TooManyWindowFunctions"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.max, ctx, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    max: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, max, ctx, diags);
        }
    }

    check_set_expr(&query.body, max, ctx, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    max: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, max, ctx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, max, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, max, ctx, diags);
            check_set_expr(right, max, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    max: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Count window functions in the projection of this SELECT.
    let count: usize = sel
        .projection
        .iter()
        .map(|item| match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                count_window_fns_in_expr(e)
            }
            _ => 0,
        })
        .sum();

    if count > max {
        let (line, col) = find_keyword_pos(&ctx.source, "SELECT");
        diags.push(Diagnostic {
            rule: "Structure/TooManyWindowFunctions",
            message: format!(
                "SELECT contains {count} window functions (max {max}); consider refactoring into CTEs"
            ),
            line,
            col,
        });
    }

    // Recurse into subqueries in FROM clause (derived tables).
    for twj in &sel.from {
        check_table_factor(&twj.relation, max, ctx, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, max, ctx, diags);
        }
    }

    // Recurse into subquery expressions in WHERE clause.
    if let Some(selection) = &sel.selection {
        check_expr_for_subqueries(selection, max, ctx, diags);
    }
}

fn check_table_factor(
    tf: &TableFactor,
    max: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, max, ctx, diags);
    }
}

/// Recursively count window functions (`Expr::Function { over: Some(_), .. }`)
/// within an expression tree.
fn count_window_fns_in_expr(expr: &Expr) -> usize {
    match expr {
        Expr::Function(func) => {
            let is_window = matches!(&func.over, Some(WindowType::WindowSpec(_)) | Some(WindowType::NamedWindow(_)));
            let self_count = if is_window { 1 } else { 0 };
            // Also count window fns in function arguments.
            self_count + count_window_fns_in_func_args(func)
        }
        Expr::BinaryOp { left, right, .. } => {
            count_window_fns_in_expr(left) + count_window_fns_in_expr(right)
        }
        Expr::UnaryOp { expr, .. } => count_window_fns_in_expr(expr),
        Expr::Nested(e) => count_window_fns_in_expr(e),
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            let op = operand.as_deref().map_or(0, count_window_fns_in_expr);
            let conds: usize = conditions.iter().map(count_window_fns_in_expr).sum();
            let res: usize = results.iter().map(count_window_fns_in_expr).sum();
            let el = else_result.as_deref().map_or(0, count_window_fns_in_expr);
            op + conds + res + el
        }
        // Subqueries in projection expressions are separate SELECTs — they will
        // be walked via check_query, not counted here.
        _ => 0,
    }
}

fn count_window_fns_in_func_args(func: &Function) -> usize {
    let FunctionArguments::List(list) = &func.args else {
        return 0;
    };
    list.args
        .iter()
        .map(|arg| {
            let fae = match arg {
                FunctionArg::Named { arg, .. }
                | FunctionArg::ExprNamed { arg, .. }
                | FunctionArg::Unnamed(arg) => arg,
            };
            if let FunctionArgExpr::Expr(e) = fae {
                count_window_fns_in_expr(e)
            } else {
                0
            }
        })
        .sum()
}

/// Walk an expression for nested subqueries only (not for window fn counting).
fn check_expr_for_subqueries(
    expr: &Expr,
    max: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Subquery(q) => check_query(q, max, ctx, diags),
        Expr::InSubquery { subquery, .. } => check_query(subquery, max, ctx, diags),
        Expr::Exists { subquery, .. } => check_query(subquery, max, ctx, diags),
        Expr::BinaryOp { left, right, .. } => {
            check_expr_for_subqueries(left, max, ctx, diags);
            check_expr_for_subqueries(right, max, ctx, diags);
        }
        _ => {}
    }
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the first occurrence of `keyword` (case-insensitive, word-boundary)
/// in `source`. Returns a 1-indexed (line, col) pair. Falls back to (1, 1).
fn find_keyword_pos(source: &str, keyword: &str) -> (usize, usize) {
    let upper = source.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let kw_len = kw_upper.len();
    let bytes = upper.as_bytes();
    let len = bytes.len();

    let mut pos = 0;
    while pos + kw_len <= len {
        let Some(rel) = upper[pos..].find(kw_upper.as_str()) else {
            break;
        };
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
