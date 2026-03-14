use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, Function, FunctionArg, FunctionArgExpr, FunctionArguments, Query, Select, SelectItem,
    SetExpr, Statement, TableFactor,
};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct NestedAggregate;

impl Rule for NestedAggregate {
    fn name(&self) -> &'static str {
        "Structure/NestedAggregate"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // Per outer-aggregate-name counter for source position lookup.
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
    select: &Select,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    // SELECT projection.
    for item in &select.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                walk_expr(e, ctx, counters, diags);
            }
            _ => {}
        }
    }

    // HAVING clause.
    if let Some(having) = &select.having {
        walk_expr(having, ctx, counters, diags);
    }

    // FROM — recurse into derived (subquery) tables.
    for table_with_joins in &select.from {
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

/// Walk an expression, looking for aggregate functions that contain aggregate
/// function calls in their arguments. Reports one violation per outer aggregate
/// that has at least one nested aggregate in its args.
fn walk_expr(
    expr: &Expr,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Function(func) => {
            let outer_name = func
                .name
                .0
                .last()
                .map(|id| id.value.to_lowercase())
                .unwrap_or_default();

            if is_aggregate(&outer_name) {
                // Check if any argument contains a nested aggregate call.
                if args_contain_aggregate(func) {
                    let name_upper = outer_name.to_uppercase();
                    let occ = counters.entry(name_upper.clone()).or_insert(0);
                    let occurrence = *occ;
                    *occ += 1;
                    let offset = find_nth_occurrence(&ctx.source, &name_upper, occurrence);
                    let (line, col) = offset_to_line_col(&ctx.source, offset);
                    diags.push(Diagnostic {
                        rule: "Structure/NestedAggregate",
                        message: "Aggregate function nested inside another aggregate — consider restructuring with a subquery or CTE".to_string(),
                        line,
                        col,
                    });
                }
                // Still walk the args to catch additional nesting levels or
                // multiple aggregate pairs in the same expression.
                walk_func_args(func, ctx, counters, diags);
            } else {
                // Non-aggregate function: walk its args for deeper detection.
                walk_func_args(func, ctx, counters, diags);
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            walk_expr(left, ctx, counters, diags);
            walk_expr(right, ctx, counters, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            walk_expr(inner, ctx, counters, diags);
        }
        Expr::Nested(inner) => {
            walk_expr(inner, ctx, counters, diags);
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                walk_expr(op, ctx, counters, diags);
            }
            for cond in conditions {
                walk_expr(cond, ctx, counters, diags);
            }
            for res in results {
                walk_expr(res, ctx, counters, diags);
            }
            if let Some(else_e) = else_result {
                walk_expr(else_e, ctx, counters, diags);
            }
        }
        _ => {}
    }
}

/// Walk the args of a function, calling `walk_expr` on each.
fn walk_func_args(
    func: &Function,
    ctx: &FileContext,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    let args = match &func.args {
        FunctionArguments::List(list) => list.args.as_slice(),
        _ => return,
    };
    for arg in args {
        let inner_expr = match arg {
            FunctionArg::Named { arg, .. }
            | FunctionArg::Unnamed(arg)
            | FunctionArg::ExprNamed { arg, .. } => match arg {
                FunctionArgExpr::Expr(e) => e,
                _ => continue,
            },
        };
        walk_expr(inner_expr, ctx, counters, diags);
    }
}

// ── Aggregate helpers ─────────────────────────────────────────────────────────

fn is_aggregate(name: &str) -> bool {
    matches!(
        name,
        "sum" | "count"
            | "avg"
            | "min"
            | "max"
            | "stddev"
            | "variance"
            | "array_agg"
            | "string_agg"
            | "listagg"
            | "group_concat"
    )
}

/// Returns `true` if any argument of `func` contains an aggregate call.
fn args_contain_aggregate(func: &Function) -> bool {
    let args = match &func.args {
        FunctionArguments::List(list) => list.args.as_slice(),
        _ => return false,
    };
    for arg in args {
        let inner_expr = match arg {
            FunctionArg::Named { arg, .. }
            | FunctionArg::Unnamed(arg)
            | FunctionArg::ExprNamed { arg, .. } => match arg {
                FunctionArgExpr::Expr(e) => e,
                _ => continue,
            },
        };
        if contains_aggregate(inner_expr) {
            return true;
        }
    }
    false
}

/// Returns `true` if `expr` is or contains an aggregate function call at any depth.
fn contains_aggregate(expr: &Expr) -> bool {
    match expr {
        Expr::Function(f) => {
            let name = f
                .name
                .0
                .last()
                .map(|id| id.value.to_lowercase())
                .unwrap_or_default();
            if is_aggregate(&name) {
                return true;
            }
            // Check inside this function's args too.
            args_contain_aggregate(f)
        }
        Expr::BinaryOp { left, right, .. } => {
            contains_aggregate(left) || contains_aggregate(right)
        }
        Expr::UnaryOp { expr: inner, .. } => contains_aggregate(inner),
        Expr::Nested(inner) => contains_aggregate(inner),
        _ => false,
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
