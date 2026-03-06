use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Function, FunctionArg, FunctionArgExpr, FunctionArguments, Query,
    Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct FunctionCallDepth {
    pub max_depth: usize,
}

impl Default for FunctionCallDepth {
    fn default() -> Self {
        FunctionCallDepth { max_depth: 3 }
    }
}

impl Rule for FunctionCallDepth {
    fn name(&self) -> &'static str {
        "FunctionCallDepth"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.max_depth, &ctx.source, &mut diags);
            }
        }
        diags
    }
}

fn check_query(query: &Query, max_depth: usize, source: &str, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, max_depth, source, diags);
        }
    }
    check_set_expr(&query.body, max_depth, source, diags);
}

fn check_set_expr(expr: &SetExpr, max_depth: usize, source: &str, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, max_depth, source, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, max_depth, source, diags);
            check_set_expr(right, max_depth, source, diags);
        }
        SetExpr::Query(inner) => check_query(inner, max_depth, source, diags),
        _ => {}
    }
}

fn check_select(sel: &Select, max_depth: usize, source: &str, diags: &mut Vec<Diagnostic>) {
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                check_top_expr(e, max_depth, source, diags);
            }
            _ => {}
        }
    }
    if let Some(selection) = &sel.selection {
        check_top_expr(selection, max_depth, source, diags);
    }
    if let Some(having) = &sel.having {
        check_top_expr(having, max_depth, source, diags);
    }
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, max_depth, source, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, max_depth, source, diags);
        }
    }
}

fn recurse_table_factor(tf: &TableFactor, max_depth: usize, source: &str, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, max_depth, source, diags);
    }
}

/// Entry point for an expression; checks if any function call chain exceeds max_depth.
fn check_top_expr(expr: &Expr, max_depth: usize, source: &str, diags: &mut Vec<Diagnostic>) {
    // Walk the expression tree. For each function call node, compute its depth
    // and report if over max_depth.
    walk_expr_for_depth(expr, max_depth, source, diags);
}

/// Returns the nesting depth of function calls starting at `expr` as the top-level root.
/// A plain function call (no nested functions) has depth 1.
/// Reports violations for any root function call that exceeds max_depth.
fn walk_expr_for_depth(expr: &Expr, max_depth: usize, source: &str, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Function(func) => {
            let depth = function_depth(expr);
            if depth > max_depth {
                let (line, col) = find_function_position(source, func);
                diags.push(Diagnostic {
                    rule: "FunctionCallDepth",
                    message: format!(
                        "Function call nesting depth {} exceeds maximum {}",
                        depth, max_depth
                    ),
                    line,
                    col,
                });
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            walk_expr_for_depth(left, max_depth, source, diags);
            walk_expr_for_depth(right, max_depth, source, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => walk_expr_for_depth(inner, max_depth, source, diags),
        Expr::Nested(inner) => walk_expr_for_depth(inner, max_depth, source, diags),
        Expr::Case { operand, conditions, results, else_result } => {
            if let Some(op) = operand { walk_expr_for_depth(op, max_depth, source, diags); }
            for c in conditions { walk_expr_for_depth(c, max_depth, source, diags); }
            for r in results { walk_expr_for_depth(r, max_depth, source, diags); }
            if let Some(e) = else_result { walk_expr_for_depth(e, max_depth, source, diags); }
        }
        _ => {}
    }
}

/// Computes the maximum function call nesting depth of a subtree.
/// `Expr::Function` at a leaf → depth 1.
/// `f(g(x))` → depth 2.
fn function_depth(expr: &Expr) -> usize {
    match expr {
        Expr::Function(func) => {
            let max_child = max_depth_in_args(func);
            1 + max_child
        }
        Expr::Nested(inner) => function_depth(inner),
        _ => 0,
    }
}

fn max_depth_in_args(func: &Function) -> usize {
    let mut max = 0usize;
    let args = match &func.args {
        FunctionArguments::List(list) => list.args.as_slice(),
        _ => return 0,
    };
    for arg in args {
        let d = match arg {
            FunctionArg::Named { arg, .. }
            | FunctionArg::Unnamed(arg)
            | FunctionArg::ExprNamed { arg, .. } => match arg {
                FunctionArgExpr::Expr(e) => function_depth(e),
                _ => 0,
            },
        };
        if d > max {
            max = d;
        }
    }
    max
}

fn find_function_position(source: &str, func: &Function) -> (usize, usize) {
    // Use the function name to find the first occurrence in source
    let name = func.name.to_string();
    find_keyword_position(source, &name)
}

fn find_keyword_position(source: &str, keyword: &str) -> (usize, usize) {
    let upper = source.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let bytes = upper.as_bytes();
    let kw_bytes = kw_upper.as_bytes();
    let kw_len = kw_bytes.len();

    if kw_len == 0 {
        return (1, 1);
    }

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

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
