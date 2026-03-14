use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, FunctionArg, FunctionArgExpr, FunctionArguments, Query, Select, SelectItem, SetExpr,
    Statement, TableFactor,
};

pub struct NoIFFunction;

const MESSAGE: &str =
    "IF() is dialect-specific (MySQL/BigQuery) \
     — use CASE WHEN ... THEN ... ELSE ... END for portable conditional logic";

/// Returns the lowercase last-ident of a function's name, or empty string.
fn func_name_lower(func: &sqlparser::ast::Function) -> String {
    func.name
        .0
        .last()
        .map(|ident| ident.value.to_lowercase())
        .unwrap_or_default()
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Find the Nth occurrence (0-indexed) of `IF` as a function call (case-insensitive)
/// in `source`. Returns byte offset or 0 if not found.
///
/// Matches `IF(` with a word boundary before so that `IFNULL` is not matched.
fn find_if_occurrence(source: &str, occurrence: usize) -> usize {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut count = 0usize;
    let mut i = 0;

    while i + 2 <= len {
        // Word boundary before.
        let before_ok = i == 0
            || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

        if before_ok {
            // Match 'I' or 'i' followed by 'F' or 'f'.
            let c0 = bytes[i];
            let c1 = bytes[i + 1];
            if (c0 == b'I' || c0 == b'i') && (c1 == b'F' || c1 == b'f') {
                // Must be followed by '(' — so the word ends here and it's a function call.
                let after = i + 2;
                if after < len && bytes[after] == b'(' {
                    if count == occurrence {
                        return i;
                    }
                    count += 1;
                }
            }
        }

        i += 1;
    }

    0
}

fn walk_expr(
    expr: &Expr,
    source: &str,
    counter: &mut usize,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Function(func) => {
            let lower = func_name_lower(func);
            if lower == "if" {
                let occ = *counter;
                *counter += 1;

                let offset = find_if_occurrence(source, occ);
                let (line, col) = line_col(source, offset);
                diags.push(Diagnostic {
                    rule,
                    message: MESSAGE.to_string(),
                    line,
                    col,
                });
            }

            // Recurse into function arguments.
            if let FunctionArguments::List(list) = &func.args {
                for arg in &list.args {
                    let inner_expr = match arg {
                        FunctionArg::Named { arg, .. }
                        | FunctionArg::Unnamed(arg)
                        | FunctionArg::ExprNamed { arg, .. } => match arg {
                            FunctionArgExpr::Expr(e) => Some(e),
                            _ => None,
                        },
                    };
                    if let Some(e) = inner_expr {
                        walk_expr(e, source, counter, rule, diags);
                    }
                }
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            walk_expr(left, source, counter, rule, diags);
            walk_expr(right, source, counter, rule, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            walk_expr(inner, source, counter, rule, diags);
        }
        Expr::Nested(inner) => {
            walk_expr(inner, source, counter, rule, diags);
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                walk_expr(op, source, counter, rule, diags);
            }
            for c in conditions {
                walk_expr(c, source, counter, rule, diags);
            }
            for r in results {
                walk_expr(r, source, counter, rule, diags);
            }
            if let Some(e) = else_result {
                walk_expr(e, source, counter, rule, diags);
            }
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    source: &str,
    counter: &mut usize,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                walk_expr(e, source, counter, rule, diags);
            }
            _ => {}
        }
    }
    if let Some(selection) = &sel.selection {
        walk_expr(selection, source, counter, rule, diags);
    }
    if let Some(having) = &sel.having {
        walk_expr(having, source, counter, rule, diags);
    }
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, source, counter, rule, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, source, counter, rule, diags);
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    source: &str,
    counter: &mut usize,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, counter, rule, diags);
    }
}

fn check_set_expr(
    expr: &SetExpr,
    source: &str,
    counter: &mut usize,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, source, counter, rule, diags),
        SetExpr::Query(inner) => check_query(inner, source, counter, rule, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, counter, rule, diags);
            check_set_expr(right, source, counter, rule, diags);
        }
        _ => {}
    }
}

fn check_query(
    query: &Query,
    source: &str,
    counter: &mut usize,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, counter, rule, diags);
        }
    }
    check_set_expr(&query.body, source, counter, rule, diags);
}

impl Rule for NoIFFunction {
    fn name(&self) -> &'static str {
        "Convention/NoIFFunction"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let mut counter = 0usize;

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &ctx.source, &mut counter, self.name(), &mut diags);
            }
        }

        diags
    }
}
