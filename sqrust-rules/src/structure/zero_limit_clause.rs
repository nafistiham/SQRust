use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor, Value};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct ZeroLimitClause;

impl Rule for ZeroLimitClause {
    fn name(&self) -> &'static str {
        "Structure/ZeroLimitClause"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // Track how many LIMIT keywords we have already reported, so that
        // when multiple queries each have LIMIT 0 we can find the Nth occurrence
        // in source text.
        let mut limit_counter: usize = 0;

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, ctx, &mut limit_counter, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    ctx: &FileContext,
    limit_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs first.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, limit_counter, diags);
        }
    }

    // Check this query's own LIMIT.
    if let Some(limit_expr) = &query.limit {
        if is_zero_literal(limit_expr) {
            let occurrence = *limit_counter;
            *limit_counter += 1;
            let (line, col) = find_nth_keyword_pos(&ctx.source, "LIMIT", occurrence);
            diags.push(Diagnostic {
                rule: "Structure/ZeroLimitClause",
                message: "LIMIT 0 always returns an empty result set".to_string(),
                line,
                col,
            });
        }
    }

    // Recurse into the body (handles nested set operations and subqueries in
    // FROM, WHERE, and SELECT).
    check_set_expr(&query.body, ctx, limit_counter, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    ctx: &FileContext,
    limit_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, ctx, limit_counter, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, ctx, limit_counter, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, limit_counter, diags);
            check_set_expr(right, ctx, limit_counter, diags);
        }
        _ => {}
    }
}

fn check_select(
    select: &Select,
    ctx: &FileContext,
    limit_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // FROM clause — check Derived (subquery) tables.
    for table_with_joins in &select.from {
        check_table_factor(&table_with_joins.relation, ctx, limit_counter, diags);
        for join in &table_with_joins.joins {
            check_table_factor(&join.relation, ctx, limit_counter, diags);
        }
    }

    // WHERE clause — check scalar subqueries.
    if let Some(selection) = &select.selection {
        check_expr_for_subqueries(selection, ctx, limit_counter, diags);
    }

    // SELECT projection — check scalar subqueries.
    for item in &select.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr_for_subqueries(e, ctx, limit_counter, diags);
        }
    }
}

fn check_table_factor(
    factor: &TableFactor,
    ctx: &FileContext,
    limit_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        check_query(subquery, ctx, limit_counter, diags);
    }
}

fn check_expr_for_subqueries(
    expr: &Expr,
    ctx: &FileContext,
    limit_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Subquery(q) => check_query(q, ctx, limit_counter, diags),
        Expr::InSubquery { subquery, .. } => check_query(subquery, ctx, limit_counter, diags),
        Expr::Exists { subquery, .. } => check_query(subquery, ctx, limit_counter, diags),
        Expr::BinaryOp { left, right, .. } => {
            check_expr_for_subqueries(left, ctx, limit_counter, diags);
            check_expr_for_subqueries(right, ctx, limit_counter, diags);
        }
        _ => {}
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns `true` when `expr` is the integer literal `0` (exactly).
fn is_zero_literal(expr: &Expr) -> bool {
    if let Expr::Value(Value::Number(s, _)) = expr {
        s == "0"
    } else {
        false
    }
}

/// Finds the `nth` (0-indexed) occurrence of `keyword` (case-insensitive,
/// word-boundary, outside strings/comments) in `source`. Returns a 1-indexed
/// (line, col) pair. Falls back to (1, 1) if not found.
fn find_nth_keyword_pos(source: &str, keyword: &str, nth: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);
    let kw_upper: Vec<u8> = keyword.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let kw_len = kw_upper.len();

    let mut count = 0usize;
    let mut i = 0usize;

    while i + kw_len <= len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }

        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(kw_upper.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if matches {
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);
            let all_code = (i..i + kw_len).all(|k| skip_map.is_code(k));

            if after_ok && all_code {
                if count == nth {
                    return offset_to_line_col(source, i);
                }
                count += 1;
            }
        }

        i += 1;
    }

    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
