use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Function, GroupByExpr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct MixedAggregateAndColumns;

impl Rule for MixedAggregateAndColumns {
    fn name(&self) -> &'static str {
        "Structure/MixedAggregateAndColumns"
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
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, ctx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, diags);
            check_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(sel: &Select, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Determine if this SELECT has no GROUP BY clause.
    let has_group_by = match &sel.group_by {
        // GROUP BY ALL is an explicit group-by; skip.
        GroupByExpr::All(_) => true,
        // Empty expressions list means no GROUP BY.
        GroupByExpr::Expressions(exprs, _) => !exprs.is_empty(),
    };

    if !has_group_by {
        // Scan projection for both aggregate functions and bare column references.
        let mut has_aggregate = false;
        let mut has_bare_column = false;

        for item in &sel.projection {
            let expr = match item {
                SelectItem::UnnamedExpr(e) => e,
                SelectItem::ExprWithAlias { expr: e, .. } => e,
                // Wildcard / QualifiedWildcard are not bare column refs for this rule.
                _ => continue,
            };
            scan_projection_expr(expr, &mut has_aggregate, &mut has_bare_column);
        }

        if has_aggregate && has_bare_column {
            let (line, col) = find_select_pos(&ctx.source);
            diags.push(Diagnostic {
                rule: "Structure/MixedAggregateAndColumns",
                message:
                    "SELECT mixes aggregate functions and non-aggregated columns without GROUP BY"
                        .to_string(),
                line,
                col,
            });
        }
    }

    // Always recurse into subqueries in FROM/JOIN.
    for twj in &sel.from {
        check_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, ctx, diags);
        }
    }

    // Recurse into subquery expressions in WHERE and SELECT projection.
    if let Some(selection) = &sel.selection {
        recurse_expr(selection, ctx, diags);
    }

    for item in &sel.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            recurse_expr(e, ctx, diags);
        }
    }
}

fn check_table_factor(tf: &TableFactor, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, ctx, diags);
    }
}

fn recurse_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Subquery(q) => check_query(q, ctx, diags),
        Expr::InSubquery { subquery, .. } => check_query(subquery, ctx, diags),
        Expr::Exists { subquery, .. } => check_query(subquery, ctx, diags),
        Expr::BinaryOp { left, right, .. } => {
            recurse_expr(left, ctx, diags);
            recurse_expr(right, ctx, diags);
        }
        _ => {}
    }
}

// ── aggregate / column detection ──────────────────────────────────────────────

/// Known aggregate function names (lower-cased).
const AGGREGATE_FUNCTIONS: &[&str] = &[
    "count",
    "sum",
    "avg",
    "min",
    "max",
    "count_if",
    "array_agg",
    "string_agg",
    "group_concat",
    "every",
    "any_value",
];

fn is_aggregate_function(func: &Function) -> bool {
    let name = func
        .name
        .0
        .last()
        .map(|i| i.value.to_lowercase())
        .unwrap_or_default();
    AGGREGATE_FUNCTIONS.contains(&name.as_str())
}

/// Scan a top-level projection expression to classify it as an aggregate call
/// and/or a bare column reference. Does NOT descend into subqueries.
fn scan_projection_expr(expr: &Expr, has_aggregate: &mut bool, has_bare_column: &mut bool) {
    match expr {
        Expr::Function(func) => {
            if is_aggregate_function(func) {
                *has_aggregate = true;
            }
            // Function arguments are not bare column refs in the projection sense.
            // We do not recurse into function args for the purpose of this rule
            // because a column inside COUNT(id) is an aggregated column.
        }
        Expr::Identifier(_) | Expr::CompoundIdentifier(_) => {
            *has_bare_column = true;
        }
        Expr::Nested(inner) => {
            scan_projection_expr(inner, has_aggregate, has_bare_column);
        }
        Expr::BinaryOp { left, right, .. } => {
            scan_projection_expr(left, has_aggregate, has_bare_column);
            scan_projection_expr(right, has_aggregate, has_bare_column);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            scan_projection_expr(inner, has_aggregate, has_bare_column);
        }
        Expr::Cast { expr: inner, .. } => {
            scan_projection_expr(inner, has_aggregate, has_bare_column);
        }
        // Literals, typed strings, wildcards, etc. are neither aggregates nor bare columns.
        _ => {}
    }
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the first occurrence of `SELECT` (word-boundary, case-insensitive)
/// in `source`. Returns a 1-indexed (line, col) pair; falls back to (1, 1).
fn find_select_pos(source: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let keyword = b"SELECT";
    let kw_len = keyword.len();

    let mut i = 0;
    while i + kw_len <= len {
        // Word boundary before.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive match.
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(keyword.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if matches {
            // Word boundary after.
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);
            if after_ok {
                return line_col(source, i);
            }
        }

        i += 1;
    }

    (1, 1)
}

/// Returns `true` if `ch` is a word character (`[a-zA-Z0-9_]`).
#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
