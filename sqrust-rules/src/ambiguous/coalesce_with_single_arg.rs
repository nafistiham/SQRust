use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, FunctionArguments, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
};

pub struct CoalesceWithSingleArg;

impl Rule for CoalesceWithSingleArg {
    fn name(&self) -> &'static str {
        "Ambiguous/CoalesceWithSingleArg"
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

/// Returns true if `expr` is `COALESCE(...)` with exactly one argument.
fn is_coalesce_single_arg(expr: &Expr) -> bool {
    if let Expr::Function(func) = expr {
        let func_name = func
            .name
            .0
            .last()
            .map(|ident| ident.value.to_uppercase())
            .unwrap_or_default();

        if func_name != "COALESCE" {
            return false;
        }

        if let FunctionArguments::List(arg_list) = &func.args {
            return arg_list.args.len() == 1;
        }
    }
    false
}

/// Recursively checks expressions for COALESCE with a single argument.
fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if is_coalesce_single_arg(expr) {
        let (line, col) = find_coalesce_pos(&ctx.source);
        diags.push(Diagnostic {
            rule: "Ambiguous/CoalesceWithSingleArg",
            message: "COALESCE with a single argument is equivalent to the argument itself — add a fallback value or remove COALESCE".to_string(),
            line,
            col,
        });
        // Still recurse into the argument in case of nested COALESCE.
        if let Expr::Function(func) = expr {
            if let FunctionArguments::List(arg_list) = &func.args {
                for arg in &arg_list.args {
                    use sqlparser::ast::FunctionArg;
                    if let FunctionArg::Unnamed(expr_arg) = arg {
                        use sqlparser::ast::FunctionArgExpr;
                        if let FunctionArgExpr::Expr(inner) = expr_arg {
                            check_expr(inner, ctx, diags);
                        }
                    }
                }
            }
        }
        return;
    }

    match expr {
        Expr::Function(func) => {
            // Recurse into all function arguments even if not COALESCE.
            if let FunctionArguments::List(arg_list) = &func.args {
                for arg in &arg_list.args {
                    use sqlparser::ast::FunctionArg;
                    if let FunctionArg::Unnamed(expr_arg) = arg {
                        use sqlparser::ast::FunctionArgExpr;
                        if let FunctionArgExpr::Expr(inner) = expr_arg {
                            check_expr(inner, ctx, diags);
                        }
                    }
                }
            }
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
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, ctx, diags);
        }
        _ => {}
    }
}

/// Finds the first occurrence of "COALESCE(" (case-insensitive) in `source`
/// and returns a 1-indexed (line, col). Falls back to (1, 1).
fn find_coalesce_pos(source: &str) -> (usize, usize) {
    find_keyword_pos(source, "COALESCE")
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
