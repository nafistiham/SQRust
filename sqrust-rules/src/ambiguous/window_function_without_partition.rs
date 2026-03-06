use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, Function, FunctionArg, FunctionArgExpr, FunctionArguments, Query, Select, SelectItem,
    SetExpr, Statement, TableFactor, WindowType,
};

pub struct WindowFunctionWithoutPartition;

impl Rule for WindowFunctionWithoutPartition {
    fn name(&self) -> &'static str {
        "Ambiguous/WindowFunctionWithoutPartition"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, ctx, &mut diags);
            }
        }
        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(query: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Visit CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, diags);
        }
    }

    check_set_expr(&query.body, ctx, diags);

    // Check ORDER BY expressions at query level.
    if let Some(order_by) = &query.order_by {
        for ob in &order_by.exprs {
            check_expr(&ob.expr, ctx, diags);
        }
    }
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, ctx, diags),
        SetExpr::Query(inner) => check_query(inner, ctx, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, diags);
            check_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(sel: &Select, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    for item in &sel.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, ctx, diags);
        }
    }

    if let Some(selection) = &sel.selection {
        check_expr(selection, ctx, diags);
    }

    if let Some(having) = &sel.having {
        check_expr(having, ctx, diags);
    }

    for twj in &sel.from {
        check_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, ctx, diags);
        }
    }
}

fn check_table_factor(tf: &TableFactor, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, ctx, diags);
    }
}

/// Recursively walk an expression to find window functions that have an
/// ORDER BY clause but no PARTITION BY clause in the window spec.
fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Function(func) => check_function(func, ctx, diags),
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, ctx, diags);
            check_expr(right, ctx, diags);
        }
        Expr::UnaryOp { expr, .. } => check_expr(expr, ctx, diags),
        Expr::Nested(e) => check_expr(e, ctx, diags),
        Expr::IsNull(e) => check_expr(e, ctx, diags),
        Expr::IsNotNull(e) => check_expr(e, ctx, diags),
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                check_expr(op, ctx, diags);
            }
            for c in conditions {
                check_expr(c, ctx, diags);
            }
            for r in results {
                check_expr(r, ctx, diags);
            }
            if let Some(el) = else_result {
                check_expr(el, ctx, diags);
            }
        }
        Expr::Subquery(q) => check_query(q, ctx, diags),
        Expr::InSubquery { subquery, .. } => check_query(subquery, ctx, diags),
        Expr::Exists { subquery, .. } => check_query(subquery, ctx, diags),
        _ => {}
    }
}

/// Check a Function node: if it has a window spec with ORDER BY but no
/// PARTITION BY, emit a diagnostic (all rows treated as one partition).
fn check_function(func: &Function, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Some(WindowType::WindowSpec(spec)) = &func.over {
        if spec.partition_by.is_empty() && !spec.order_by.is_empty() {
            let (line, col) = find_func_name_pos(&ctx.source, &func.name.to_string());
            diags.push(Diagnostic {
                rule: "Ambiguous/WindowFunctionWithoutPartition",
                message:
                    "Window function has ORDER BY but no PARTITION BY; all rows are treated as one partition"
                        .to_string(),
                line,
                col,
            });
        }
    }

    // Recurse into function arguments — they can contain window functions too.
    if let FunctionArguments::List(list) = &func.args {
        for arg in &list.args {
            let fae = match arg {
                FunctionArg::Named { arg, .. }
                | FunctionArg::ExprNamed { arg, .. }
                | FunctionArg::Unnamed(arg) => arg,
            };
            if let FunctionArgExpr::Expr(e) = fae {
                check_expr(e, ctx, diags);
            }
        }
    }
}

// ── position helpers ──────────────────────────────────────────────────────────

/// Scan source for the first word-boundary occurrence of `name`
/// (case-insensitive) and return its 1-indexed (line, col). Falls back to (1, 1).
fn find_func_name_pos(source: &str, name: &str) -> (usize, usize) {
    // Use only the last identifier component (strip schema qualifiers).
    let bare = name.rsplit('.').next().unwrap_or(name);
    let upper_name = bare.to_uppercase();
    let kw_len = upper_name.len();
    let upper_src = source.to_uppercase();
    let bytes = upper_src.as_bytes();
    let len = bytes.len();

    let mut pos = 0;
    while pos + kw_len <= len {
        if let Some(rel) = upper_src[pos..].find(upper_name.as_str()) {
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
