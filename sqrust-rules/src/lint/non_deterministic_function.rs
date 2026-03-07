use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, FunctionArg, FunctionArgExpr, FunctionArguments, Query, Select, SelectItem, SetExpr,
    Statement, TableFactor,
};

pub struct NonDeterministicFunction;

/// Function names (uppercased) that are considered non-deterministic.
const NON_DETERMINISTIC: &[&str] = &[
    "RAND",
    "RANDOM",
    "UUID",
    "NEWID",
    "NEWSEQUENTIALID",
    "GEN_RANDOM_UUID",
];

impl Rule for NonDeterministicFunction {
    fn name(&self) -> &'static str {
        "Lint/NonDeterministicFunction"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST may be incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // Per-function-name occurrence counter so that find_occurrence can
        // locate the correct position in source when a name appears multiple times.
        let mut occurrence_counters: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for stmt in &ctx.statements {
            walk_statement(stmt, &ctx.source, &mut occurrence_counters, &mut diags);
        }

        diags
    }
}

// ── Statement walker ──────────────────────────────────────────────────────────

fn walk_statement(
    stmt: &Statement,
    source: &str,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    match stmt {
        Statement::Query(q) => walk_query(q, source, counters, diags),
        Statement::Insert(insert) => {
            if let Some(src) = &insert.source {
                walk_query(src, source, counters, diags);
            }
        }
        Statement::Update {
            selection, assignments, ..
        } => {
            if let Some(expr) = selection {
                walk_expr(expr, source, counters, diags);
            }
            for assign in assignments {
                walk_expr(&assign.value, source, counters, diags);
            }
        }
        Statement::Delete(delete) => {
            if let Some(expr) = &delete.selection {
                walk_expr(expr, source, counters, diags);
            }
        }
        _ => {}
    }
}

// ── Query / SET-expression walker ─────────────────────────────────────────────

fn walk_query(
    query: &Query,
    source: &str,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            walk_query(&cte.query, source, counters, diags);
        }
    }
    walk_set_expr(&query.body, source, counters, diags);
}

fn walk_set_expr(
    expr: &SetExpr,
    source: &str,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => walk_select(sel, source, counters, diags),
        SetExpr::Query(inner) => walk_query(inner, source, counters, diags),
        SetExpr::SetOperation { left, right, .. } => {
            walk_set_expr(left, source, counters, diags);
            walk_set_expr(right, source, counters, diags);
        }
        _ => {}
    }
}

fn walk_select(
    sel: &Select,
    source: &str,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    // Projection
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                walk_expr(e, source, counters, diags);
            }
            _ => {}
        }
    }

    // WHERE clause
    if let Some(expr) = &sel.selection {
        walk_expr(expr, source, counters, diags);
    }

    // HAVING clause
    if let Some(expr) = &sel.having {
        walk_expr(expr, source, counters, diags);
    }

    // GROUP BY expressions
    if let sqlparser::ast::GroupByExpr::Expressions(exprs, _) = &sel.group_by {
        for e in exprs {
            walk_expr(e, source, counters, diags);
        }
    }

    // Subqueries inside FROM
    for twj in &sel.from {
        walk_table_factor(&twj.relation, source, counters, diags);
        for join in &twj.joins {
            walk_table_factor(&join.relation, source, counters, diags);
        }
    }
}

fn walk_table_factor(
    tf: &TableFactor,
    source: &str,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        walk_query(subquery, source, counters, diags);
    }
}

// ── Expression walker ─────────────────────────────────────────────────────────

fn walk_expr(
    expr: &Expr,
    source: &str,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Function(func) => {
            // Extract the last ident in the function name (handles schema-qualified calls)
            let name_upper = func
                .name
                .0
                .last()
                .map(|ident| ident.value.to_uppercase())
                .unwrap_or_default();

            if NON_DETERMINISTIC.contains(&name_upper.as_str()) {
                let occ = counters.entry(name_upper.clone()).or_insert(0);
                let occurrence = *occ;
                *occ += 1;

                let offset = find_occurrence(source, &name_upper, occurrence);
                let (line, col) = offset_to_line_col(source, offset);

                diags.push(Diagnostic {
                    rule: "Lint/NonDeterministicFunction",
                    message: format!(
                        "Non-deterministic function {}() produces different results on each call",
                        name_upper
                    ),
                    line,
                    col,
                });
            }

            // Recurse into function arguments
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
                        walk_expr(e, source, counters, diags);
                    }
                }
            }
        }

        Expr::BinaryOp { left, right, .. } => {
            walk_expr(left, source, counters, diags);
            walk_expr(right, source, counters, diags);
        }

        Expr::UnaryOp { expr: inner, .. } => {
            walk_expr(inner, source, counters, diags);
        }

        Expr::Nested(inner) => walk_expr(inner, source, counters, diags),

        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                walk_expr(op, source, counters, diags);
            }
            for c in conditions {
                walk_expr(c, source, counters, diags);
            }
            for r in results {
                walk_expr(r, source, counters, diags);
            }
            if let Some(e) = else_result {
                walk_expr(e, source, counters, diags);
            }
        }

        Expr::InList {
            expr: inner,
            list,
            ..
        } => {
            walk_expr(inner, source, counters, diags);
            for e in list {
                walk_expr(e, source, counters, diags);
            }
        }

        Expr::InSubquery {
            expr: inner,
            subquery,
            ..
        } => {
            walk_expr(inner, source, counters, diags);
            walk_query(subquery, source, counters, diags);
        }

        Expr::Exists { subquery, .. } => walk_query(subquery, source, counters, diags),

        Expr::Subquery(q) => walk_query(q, source, counters, diags),

        Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
            walk_expr(inner, source, counters, diags);
        }

        Expr::Between {
            expr: inner,
            low,
            high,
            ..
        } => {
            walk_expr(inner, source, counters, diags);
            walk_expr(low, source, counters, diags);
            walk_expr(high, source, counters, diags);
        }

        Expr::Like {
            expr: inner,
            pattern,
            ..
        }
        | Expr::ILike {
            expr: inner,
            pattern,
            ..
        } => {
            walk_expr(inner, source, counters, diags);
            walk_expr(pattern, source, counters, diags);
        }

        // Literals, identifiers, wildcards, etc. — nothing to recurse into
        _ => {}
    }
}

// ── Source-text helpers ───────────────────────────────────────────────────────

/// Finds the byte offset of the `nth` (0-indexed) whole-word, case-insensitive
/// occurrence of `name` (uppercased) in `source`. Returns 0 if not found.
fn find_occurrence(source: &str, name: &str, nth: usize) -> usize {
    let bytes = source.as_bytes();
    let name_bytes: Vec<u8> = name.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let name_len = name_bytes.len();
    let src_len = bytes.len();

    let mut count = 0usize;
    let mut i = 0usize;

    while i + name_len <= src_len {
        let before_ok = i == 0
            || {
                let b = bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

        if before_ok {
            let matches = bytes[i..i + name_len]
                .iter()
                .zip(name_bytes.iter())
                .all(|(&a, &b)| a.to_ascii_uppercase() == b);

            if matches {
                let after = i + name_len;
                let after_ok = after >= src_len
                    || {
                        let b = bytes[after];
                        !b.is_ascii_alphanumeric() && b != b'_'
                    };

                if after_ok {
                    if count == nth {
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

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
