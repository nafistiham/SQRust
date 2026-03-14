use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SetExpr, Statement, TableFactor, WindowType};

pub struct WindowFunctionInWhere;

impl Rule for WindowFunctionInWhere {
    fn name(&self) -> &'static str {
        "Structure/WindowFunctionInWhere"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.name(), &ctx.source, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    // Recurse into CTEs first.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, source, diags);
        }
    }
    check_set_expr(&query.body, rule, source, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, rule, source, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, rule, source, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, rule, source, diags);
            check_set_expr(right, rule, source, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    // Check WHERE clause for window functions.
    if let Some(selection) = &sel.selection {
        collect_window_fns_in_expr(selection, rule, source, diags);
    }

    // Recurse into derived tables in FROM clause.
    for twj in &sel.from {
        check_table_factor(&twj.relation, rule, source, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, rule, source, diags);
        }
    }
}

fn check_table_factor(
    tf: &TableFactor,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, rule, source, diags);
    }
}

// ── Window function detection in expressions ──────────────────────────────────

/// Recursively walk `expr` and emit a Diagnostic for each window function found.
/// A window function is an `Expr::Function` with a non-None `over` field
/// (`WindowType::WindowSpec` or `WindowType::NamedWindow`).
fn collect_window_fns_in_expr(
    expr: &Expr,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Function(func) => {
            let is_window = matches!(
                &func.over,
                Some(WindowType::WindowSpec(_)) | Some(WindowType::NamedWindow(_))
            );
            if is_window {
                let func_name = func
                    .name
                    .0
                    .last()
                    .map(|ident| ident.value.as_str())
                    .unwrap_or("window function");
                let (line, col) = find_function_pos(source, func_name);
                diags.push(Diagnostic {
                    rule,
                    message: format!(
                        "Window function '{func_name}' used in WHERE clause \
                         — window functions cannot be used in WHERE; \
                         wrap in a subquery or CTE and filter on the result"
                    ),
                    line,
                    col,
                });
            }
            // Do not recurse into function arguments here; nested window
            // functions inside a non-window function are separate concerns.
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_window_fns_in_expr(left, rule, source, diags);
            collect_window_fns_in_expr(right, rule, source, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            collect_window_fns_in_expr(inner, rule, source, diags);
        }
        Expr::Nested(inner) => {
            collect_window_fns_in_expr(inner, rule, source, diags);
        }
        Expr::Between {
            expr: e,
            low,
            high,
            ..
        } => {
            collect_window_fns_in_expr(e, rule, source, diags);
            collect_window_fns_in_expr(low, rule, source, diags);
            collect_window_fns_in_expr(high, rule, source, diags);
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                collect_window_fns_in_expr(op, rule, source, diags);
            }
            for cond in conditions {
                collect_window_fns_in_expr(cond, rule, source, diags);
            }
            for res in results {
                collect_window_fns_in_expr(res, rule, source, diags);
            }
            if let Some(else_e) = else_result {
                collect_window_fns_in_expr(else_e, rule, source, diags);
            }
        }
        Expr::InList { expr: inner, list, .. } => {
            collect_window_fns_in_expr(inner, rule, source, diags);
            for e in list {
                collect_window_fns_in_expr(e, rule, source, diags);
            }
        }
        _ => {}
    }
}

// ── Source-text position helpers ──────────────────────────────────────────────

/// Find the first occurrence of `func_name` (case-insensitive, word-boundary)
/// in `source` that looks like a function call (followed by `(`).
/// Falls back to (1, 1).
fn find_function_pos(source: &str, func_name: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let upper = source.to_uppercase();
    let name_upper = func_name.to_uppercase();
    let name_len = name_upper.len();
    let len = bytes.len();

    let mut i = 0usize;
    while i + name_len <= len {
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if before_ok {
            let slice = &upper[i..];
            if slice.starts_with(name_upper.as_str()) {
                let after = i + name_len;
                let after_ok = after >= len || !is_word_char(bytes[after]);
                if after_ok {
                    return offset_to_line_col(source, i);
                }
            }
        }
        i += 1;
    }

    (1, 1)
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
