use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, FunctionArg, FunctionArgExpr, FunctionArguments, Query, Select, SelectItem, SetExpr,
    Statement, TableFactor,
};

pub struct GetDate;

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
                let after = i + name_len;
                // Must be followed by '(' to be a function call and have a word boundary.
                let after_ok = after < len
                    && bytes[after] == b'('
                    && (after == name_len
                        || !bytes[i + name_len - 1].is_ascii_alphanumeric());
                // Also verify no word character immediately after the name (already guaranteed
                // by checking for '('), but also that the match is a complete word.
                let word_end_ok = after >= len || !bytes[after - 1 + 1].is_ascii_alphabetic()
                    || bytes[after] == b'(';
                let _ = word_end_ok; // used via after_ok above

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

fn message_for(name: &str) -> String {
    match name {
        "getdate" => {
            "GETDATE() is SQL Server-specific — use CURRENT_TIMESTAMP for portable current timestamp"
                .to_string()
        }
        "getutcdate" => {
            "GETUTCDATE() is SQL Server-specific — use CURRENT_TIMESTAMP AT TIME ZONE 'UTC' or a dialect-specific equivalent"
                .to_string()
        }
        _ => unreachable!(),
    }
}

fn walk_expr(
    expr: &Expr,
    source: &str,
    counters: &mut [usize; 2],
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Function(func) => {
            let lower = func_name_lower(func);
            let func_key = match lower.as_str() {
                "getdate" => Some(0usize),
                "getutcdate" => Some(1usize),
                _ => None,
            };
            if let Some(idx) = func_key {
                let occ = counters[idx];
                counters[idx] += 1;

                let func_name_str = if idx == 0 { "GETDATE" } else { "GETUTCDATE" };
                let offset = find_occurrence(source, func_name_str, occ);
                let (line, col) = line_col(source, offset);
                diags.push(Diagnostic {
                    rule,
                    message: message_for(lower.as_str()),
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
                        walk_expr(e, source, counters, rule, diags);
                    }
                }
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            walk_expr(left, source, counters, rule, diags);
            walk_expr(right, source, counters, rule, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            walk_expr(inner, source, counters, rule, diags);
        }
        Expr::Nested(inner) => {
            walk_expr(inner, source, counters, rule, diags);
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                walk_expr(op, source, counters, rule, diags);
            }
            for c in conditions {
                walk_expr(c, source, counters, rule, diags);
            }
            for r in results {
                walk_expr(r, source, counters, rule, diags);
            }
            if let Some(e) = else_result {
                walk_expr(e, source, counters, rule, diags);
            }
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    source: &str,
    counters: &mut [usize; 2],
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                walk_expr(e, source, counters, rule, diags);
            }
            _ => {}
        }
    }
    if let Some(selection) = &sel.selection {
        walk_expr(selection, source, counters, rule, diags);
    }
    if let Some(having) = &sel.having {
        walk_expr(having, source, counters, rule, diags);
    }
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, source, counters, rule, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, source, counters, rule, diags);
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    source: &str,
    counters: &mut [usize; 2],
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, counters, rule, diags);
    }
}

fn check_set_expr(
    expr: &SetExpr,
    source: &str,
    counters: &mut [usize; 2],
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, source, counters, rule, diags),
        SetExpr::Query(inner) => check_query(inner, source, counters, rule, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, counters, rule, diags);
            check_set_expr(right, source, counters, rule, diags);
        }
        _ => {}
    }
}

fn check_query(
    query: &Query,
    source: &str,
    counters: &mut [usize; 2],
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, counters, rule, diags);
        }
    }
    check_set_expr(&query.body, source, counters, rule, diags);
}

impl Rule for GetDate {
    fn name(&self) -> &'static str {
        "Convention/GetDate"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // counters[0] = getdate occurrences, counters[1] = getutcdate occurrences
        let mut counters = [0usize; 2];

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &ctx.source, &mut counters, self.name(), &mut diags);
            }
        }

        diags
    }
}
