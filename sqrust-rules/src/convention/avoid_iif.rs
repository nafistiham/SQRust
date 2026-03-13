use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, FunctionArg, FunctionArgExpr, FunctionArguments, Query, Select, SelectItem, SetExpr,
    Statement, TableFactor,
};

pub struct AvoidIif;

/// Returns the uppercase last-ident of a function's name, or empty string.
fn func_name_upper(func: &sqlparser::ast::Function) -> String {
    func.name
        .0
        .last()
        .map(|ident| ident.value.to_uppercase())
        .unwrap_or_default()
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Find the Nth occurrence (0-indexed) of `name` as a function call (case-insensitive)
/// in `source`. Returns byte offset or 0 if not found.
fn find_occurrence(source: &str, name: &str, occurrence: usize) -> usize {
    let bytes = source.as_bytes();
    let name_upper: Vec<u8> = name.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let name_len = name_upper.len();
    let len = bytes.len();
    let mut count = 0usize;
    let mut i = 0;

    while i + name_len <= len {
        // Word boundary before.
        let before_ok = i == 0
            || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

        if before_ok {
            let matches = bytes[i..i + name_len]
                .iter()
                .zip(name_upper.iter())
                .all(|(&a, &b)| a.eq_ignore_ascii_case(&b));

            if matches {
                // Word boundary after (must be followed by '(' to be a function call).
                let after = i + name_len;
                let after_ok = after < len && bytes[after] == b'(';

                if after_ok {
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

/// Walk an expression, pushing diagnostics for any IIF() function call.
fn walk_expr(
    expr: &Expr,
    source: &str,
    occurrence_counter: &mut usize,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Function(func) => {
            let upper = func_name_upper(func);
            if upper == "IIF" {
                let occ = *occurrence_counter;
                *occurrence_counter += 1;

                let offset = find_occurrence(source, "IIF", occ);
                let (line, col) = line_col(source, offset);
                diags.push(Diagnostic {
                    rule,
                    message: "IIF() is SQL Server-specific — use CASE WHEN condition THEN value ELSE other END for portability".to_string(),
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
                        walk_expr(e, source, occurrence_counter, rule, diags);
                    }
                }
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            walk_expr(left, source, occurrence_counter, rule, diags);
            walk_expr(right, source, occurrence_counter, rule, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            walk_expr(inner, source, occurrence_counter, rule, diags);
        }
        Expr::Nested(inner) => {
            walk_expr(inner, source, occurrence_counter, rule, diags);
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                walk_expr(op, source, occurrence_counter, rule, diags);
            }
            for c in conditions {
                walk_expr(c, source, occurrence_counter, rule, diags);
            }
            for r in results {
                walk_expr(r, source, occurrence_counter, rule, diags);
            }
            if let Some(e) = else_result {
                walk_expr(e, source, occurrence_counter, rule, diags);
            }
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    source: &str,
    occurrence_counter: &mut usize,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    // Projection.
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                walk_expr(e, source, occurrence_counter, rule, diags);
            }
            _ => {}
        }
    }
    // WHERE clause.
    if let Some(selection) = &sel.selection {
        walk_expr(selection, source, occurrence_counter, rule, diags);
    }
    // HAVING clause.
    if let Some(having) = &sel.having {
        walk_expr(having, source, occurrence_counter, rule, diags);
    }
    // Recurse into subqueries in FROM.
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, source, occurrence_counter, rule, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, source, occurrence_counter, rule, diags);
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    source: &str,
    occurrence_counter: &mut usize,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, occurrence_counter, rule, diags);
    }
}

fn check_set_expr(
    expr: &SetExpr,
    source: &str,
    occurrence_counter: &mut usize,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, source, occurrence_counter, rule, diags),
        SetExpr::Query(inner) => check_query(inner, source, occurrence_counter, rule, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, occurrence_counter, rule, diags);
            check_set_expr(right, source, occurrence_counter, rule, diags);
        }
        _ => {}
    }
}

fn check_query(
    query: &Query,
    source: &str,
    occurrence_counter: &mut usize,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, occurrence_counter, rule, diags);
        }
    }
    check_set_expr(&query.body, source, occurrence_counter, rule, diags);
}

impl Rule for AvoidIif {
    fn name(&self) -> &'static str {
        "Convention/AvoidIif"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // AST-based — return empty if the file did not parse.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let mut occurrence_counter = 0usize;

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(
                    query,
                    &ctx.source,
                    &mut occurrence_counter,
                    self.name(),
                    &mut diags,
                );
            }
        }

        diags
    }
}
