use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct OrderByInSubquery;

impl Rule for OrderByInSubquery {
    fn name(&self) -> &'static str {
        "OrderByInSubquery"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                // Walk the top-level query, but exempt it from the check.
                // CTEs inside WITH are inner queries → checked.
                // The body of the top-level query is walked for subqueries,
                // but the top-level query itself (query.order_by) is not flagged.
                check_top_level_query(query, ctx, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

/// Walk the top-level query:
/// - CTEs are inner queries → pass to `check_inner_query`
/// - The body is walked for nested subqueries
fn check_top_level_query(query: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Walk CTEs — each CTE body is an inner query.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_inner_query(&cte.query, ctx, diags);
        }
    }

    // Walk the body for any nested subqueries (not the top-level body itself).
    check_set_expr_for_subqueries(&query.body, ctx, diags);
}

/// Check an inner (subquery) Query: flag if it has ORDER BY without LIMIT/OFFSET,
/// then recurse into its own subqueries.
fn check_inner_query(query: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Flag if ORDER BY present and no LIMIT and no OFFSET.
    if let Some(order_by) = &query.order_by {
        if !order_by.exprs.is_empty() && query.limit.is_none() && query.offset.is_none() {
            let (line, col) = find_keyword_pos(&ctx.source, "ORDER BY");
            diags.push(Diagnostic {
                rule: "OrderByInSubquery",
                message: "ORDER BY in subquery without LIMIT has no effect on the final result"
                    .to_string(),
                line,
                col,
            });
        }
    }

    // Recurse into CTEs of this inner query.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_inner_query(&cte.query, ctx, diags);
        }
    }

    // Recurse into the body for further nested subqueries.
    check_set_expr_for_subqueries(&query.body, ctx, diags);
}

/// Walk a SetExpr looking for subqueries inside it (not the SetExpr itself).
fn check_set_expr_for_subqueries(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => check_select_for_subqueries(sel, ctx, diags),
        SetExpr::Query(inner) => check_inner_query(inner, ctx, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr_for_subqueries(left, ctx, diags);
            check_set_expr_for_subqueries(right, ctx, diags);
        }
        _ => {}
    }
}

/// Walk a SELECT clause looking for subqueries in FROM, WHERE, and projections.
fn check_select_for_subqueries(sel: &Select, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    for twj in &sel.from {
        // Derived table (subquery in FROM).
        check_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, ctx, diags);
        }
    }

    // WHERE clause.
    if let Some(selection) = &sel.selection {
        check_expr(selection, ctx, diags);
    }

    // SELECT list.
    for item in &sel.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, ctx, diags);
        }
    }
}

fn check_table_factor(tf: &TableFactor, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_inner_query(subquery, ctx, diags);
    }
}

fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Subquery(q) => check_inner_query(q, ctx, diags),
        Expr::InSubquery { subquery, .. } => check_inner_query(subquery, ctx, diags),
        Expr::Exists { subquery, .. } => check_inner_query(subquery, ctx, diags),
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, ctx, diags);
            check_expr(right, ctx, diags);
        }
        _ => {}
    }
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the first occurrence of a two-word keyword like "ORDER BY"
/// (case-insensitive, word-boundary on the first word) in `source`, outside
/// strings and comments.
///
/// Returns a 1-indexed (line, col) pair. Falls back to (1, 1) if not found.
fn find_keyword_pos(source: &str, keyword: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);
    let kw_upper: Vec<u8> = keyword.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let kw_len = kw_upper.len();

    let mut i = 0;
    while i + kw_len <= len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }

        // Word boundary before first character.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive multi-character match (including internal space).
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(kw_upper.iter())
            .all(|(a, b)| {
                // The space character in "ORDER BY" is not a word char, just match exactly.
                if *b == b' ' {
                    *a == b' ' || *a == b'\t'
                } else {
                    a.eq_ignore_ascii_case(b)
                }
            });

        if matches {
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);
            // All non-space bytes must be real code.
            let all_code = (i..i + kw_len).all(|k| kw_upper[k - i] == b' ' || skip_map.is_code(k));

            if after_ok && all_code {
                return line_col(source, i);
            }
        }

        i += 1;
    }

    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
