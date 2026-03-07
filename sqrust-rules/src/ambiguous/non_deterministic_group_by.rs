use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, GroupByExpr, Query, SetExpr, Statement, TableFactor};

pub struct NonDeterministicGroupBy;

/// Function names (lowercase) that are considered non-deterministic.
const NON_DETERMINISTIC: &[&str] = &[
    "rand",
    "random",
    "uuid",
    "uuid4",
    "newid",
    "gen_random_uuid",
    "sys_guid",
];

impl Rule for NonDeterministicGroupBy {
    fn name(&self) -> &'static str {
        "Ambiguous/NonDeterministicGroupBy"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // Per-function-name occurrence counter to locate the correct position
        // when a name appears multiple times in source.
        let mut counters: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for stmt in &ctx.statements {
            collect_from_statement(stmt, ctx, &mut counters, &mut diags);
        }
        diags
    }
}

fn collect_from_statement(
    stmt: &Statement,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    if let Statement::Query(query) = stmt {
        collect_from_query(query, ctx, counters, diags);
    }
}

fn collect_from_query(
    query: &Query,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    // Recurse into CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, ctx, counters, diags);
        }
    }
    collect_from_set_expr(&query.body, ctx, counters, diags);
}

fn collect_from_set_expr(
    expr: &SetExpr,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(select) => {
            // Only check GROUP BY expressions — not SELECT, WHERE, or HAVING.
            if let GroupByExpr::Expressions(exprs, _) = &select.group_by {
                for group_expr in exprs {
                    check_expr_for_non_deterministic(group_expr, ctx, counters, diags);
                }
            }
            // GroupByExpr::All(_) — skip; not a per-row expression.

            // Recurse into derived tables in FROM.
            for twj in &select.from {
                collect_from_table_factor(&twj.relation, ctx, counters, diags);
                for join in &twj.joins {
                    collect_from_table_factor(&join.relation, ctx, counters, diags);
                }
            }
        }
        SetExpr::Query(inner) => {
            collect_from_query(inner, ctx, counters, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            collect_from_set_expr(left, ctx, counters, diags);
            collect_from_set_expr(right, ctx, counters, diags);
        }
        _ => {}
    }
}

fn collect_from_table_factor(
    factor: &TableFactor,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        collect_from_query(subquery, ctx, counters, diags);
    }
}

/// Recursively walks `expr` looking for calls to non-deterministic functions.
/// Emits one diagnostic per function call found.
fn check_expr_for_non_deterministic(
    expr: &Expr,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Function(func) => {
            let name_lower = func
                .name
                .0
                .last()
                .map(|ident| ident.value.to_lowercase())
                .unwrap_or_default();

            if NON_DETERMINISTIC.contains(&name_lower.as_str()) {
                // Use the upper-cased version for source lookup and message.
                let name_upper = name_lower.to_uppercase();
                let occ = counters.entry(name_upper.clone()).or_insert(0);
                let occurrence = *occ;
                *occ += 1;

                let offset = find_occurrence(&ctx.source, &name_upper, occurrence);
                let (line, col) = offset_to_line_col(&ctx.source, offset);

                diags.push(Diagnostic {
                    rule: "Ambiguous/NonDeterministicGroupBy",
                    message:
                        "Non-deterministic function in GROUP BY makes grouping unpredictable"
                            .to_string(),
                    line,
                    col,
                });
            }

            // Recurse into function arguments — handles f(RAND()) style expressions.
            use sqlparser::ast::{FunctionArg, FunctionArgExpr, FunctionArguments};
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
                        check_expr_for_non_deterministic(e, ctx, counters, diags);
                    }
                }
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_expr_for_non_deterministic(left, ctx, counters, diags);
            check_expr_for_non_deterministic(right, ctx, counters, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr_for_non_deterministic(inner, ctx, counters, diags);
        }
        Expr::Nested(inner) => {
            check_expr_for_non_deterministic(inner, ctx, counters, diags);
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                check_expr_for_non_deterministic(op, ctx, counters, diags);
            }
            for c in conditions {
                check_expr_for_non_deterministic(c, ctx, counters, diags);
            }
            for r in results {
                check_expr_for_non_deterministic(r, ctx, counters, diags);
            }
            if let Some(e) = else_result {
                check_expr_for_non_deterministic(e, ctx, counters, diags);
            }
        }
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
