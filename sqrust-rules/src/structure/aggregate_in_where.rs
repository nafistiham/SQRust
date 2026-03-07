use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct AggregateInWhere;

/// Aggregate function names (uppercased) that are forbidden in a WHERE clause.
const AGGREGATES: &[&str] = &[
    "COUNT",
    "SUM",
    "AVG",
    "MIN",
    "MAX",
    "ARRAY_AGG",
    "STRING_AGG",
    "GROUP_CONCAT",
    "EVERY",
    "COUNT_IF",
    "ANY_VALUE",
];

impl Rule for AggregateInWhere {
    fn name(&self) -> &'static str {
        "Structure/AggregateInWhere"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // Per-function-name occurrence counter so `find_nth_occurrence` can
        // locate the correct source position when a name appears multiple times.
        let mut counters: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, ctx, &mut counters, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, counters, diags);
        }
    }
    check_set_expr(&query.body, ctx, counters, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, ctx, counters, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, ctx, counters, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, counters, diags);
            check_set_expr(right, ctx, counters, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    // Check the WHERE clause for aggregate functions.
    if let Some(selection) = &sel.selection {
        collect_aggregates_in_expr(selection, ctx, counters, diags);
    }

    // Recurse into subqueries in the FROM clause.
    for table_with_joins in &sel.from {
        recurse_table_factor(&table_with_joins.relation, ctx, counters, diags);
        for join in &table_with_joins.joins {
            recurse_table_factor(&join.relation, ctx, counters, diags);
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, ctx, counters, diags);
    }
}

// ── Aggregate detection in expressions ───────────────────────────────────────

/// Recursively walks `expr` and emits a Diagnostic for every aggregate function
/// call found directly inside the expression.
fn collect_aggregates_in_expr(
    expr: &Expr,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Function(func) => {
            let name_upper = func
                .name
                .0
                .last()
                .map(|ident| ident.value.to_uppercase())
                .unwrap_or_default();

            if AGGREGATES.contains(&name_upper.as_str()) {
                let occ = counters.entry(name_upper.clone()).or_insert(0);
                let occurrence = *occ;
                *occ += 1;

                let offset = find_nth_occurrence(&ctx.source, &name_upper, occurrence);
                let (line, col) = offset_to_line_col(&ctx.source, offset);

                diags.push(Diagnostic {
                    rule: "Structure/AggregateInWhere",
                    message: "Aggregate function in WHERE clause; use HAVING instead".to_string(),
                    line,
                    col,
                });
            }
            // Do not recurse into function args — nested aggregates inside an
            // aggregate arg are a different issue and the outer call is what
            // sits in WHERE.
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_aggregates_in_expr(left, ctx, counters, diags);
            collect_aggregates_in_expr(right, ctx, counters, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            collect_aggregates_in_expr(inner, ctx, counters, diags);
        }
        Expr::Nested(inner) => {
            collect_aggregates_in_expr(inner, ctx, counters, diags);
        }
        Expr::Between {
            expr: e,
            low,
            high,
            ..
        } => {
            collect_aggregates_in_expr(e, ctx, counters, diags);
            collect_aggregates_in_expr(low, ctx, counters, diags);
            collect_aggregates_in_expr(high, ctx, counters, diags);
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                collect_aggregates_in_expr(op, ctx, counters, diags);
            }
            for cond in conditions {
                collect_aggregates_in_expr(cond, ctx, counters, diags);
            }
            for res in results {
                collect_aggregates_in_expr(res, ctx, counters, diags);
            }
            if let Some(else_e) = else_result {
                collect_aggregates_in_expr(else_e, ctx, counters, diags);
            }
        }
        Expr::InList { expr: inner, list, .. } => {
            collect_aggregates_in_expr(inner, ctx, counters, diags);
            for e in list {
                collect_aggregates_in_expr(e, ctx, counters, diags);
            }
        }
        Expr::InSubquery {
            expr: inner,
            subquery,
            ..
        } => {
            collect_aggregates_in_expr(inner, ctx, counters, diags);
            check_query(subquery, ctx, counters, diags);
        }
        Expr::Exists { subquery, .. } => {
            check_query(subquery, ctx, counters, diags);
        }
        Expr::Subquery(q) => {
            check_query(q, ctx, counters, diags);
        }
        _ => {}
    }
}

// ── Source-text helpers ───────────────────────────────────────────────────────

/// Finds the byte offset of the `nth` (0-indexed) whole-word,
/// case-insensitive occurrence of `name` (already uppercased) in `source`,
/// skipping positions inside strings/comments. Returns 0 if not found.
fn find_nth_occurrence(source: &str, name: &str, nth: usize) -> usize {
    let bytes = source.as_bytes();
    let skip_map = SkipMap::build(source);
    let name_bytes: Vec<u8> = name.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let name_len = name_bytes.len();
    let src_len = bytes.len();

    let mut count = 0usize;
    let mut i = 0usize;

    while i + name_len <= src_len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }

        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        let matches = bytes[i..i + name_len]
            .iter()
            .zip(name_bytes.iter())
            .all(|(&a, &b)| a.to_ascii_uppercase() == b);

        if matches {
            let after = i + name_len;
            let after_ok = after >= src_len || !is_word_char(bytes[after]);
            let all_code = (i..i + name_len).all(|k| skip_map.is_code(k));

            if after_ok && all_code {
                if count == nth {
                    return i;
                }
                count += 1;
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
