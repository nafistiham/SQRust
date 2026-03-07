use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct TooManySubqueries {
    /// Maximum number of subqueries allowed in a single SQL statement.
    /// `Expr::Subquery`, `Expr::InSubquery`, `Expr::Exists`, and each CTE
    /// all count as one subquery each.
    pub max_subqueries: usize,
}

impl Default for TooManySubqueries {
    fn default() -> Self {
        TooManySubqueries { max_subqueries: 3 }
    }
}

impl Rule for TooManySubqueries {
    fn name(&self) -> &'static str {
        "Structure/TooManySubqueries"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                let n = count_subqueries_in_query(query);
                if n > self.max_subqueries {
                    let (line, col) = find_nth_select_pos(&ctx.source, 1);
                    diags.push(Diagnostic {
                        rule: "Structure/TooManySubqueries",
                        message: format!(
                            "Statement contains {n} subqueries, exceeding the maximum of {max}",
                            max = self.max_subqueries,
                        ),
                        line,
                        col,
                    });
                }
            }
        }

        diags
    }
}

// ── subquery counting ─────────────────────────────────────────────────────────

/// Count all subqueries in a Query node, including its CTEs.
/// Each CTE body counts as one subquery, plus any inline subquery expressions.
fn count_subqueries_in_query(query: &Query) -> usize {
    let mut count = 0;

    // Each CTE counts as one subquery.
    if let Some(with) = &query.with {
        count += with.cte_tables.len();
        // Also count subqueries nested inside each CTE body.
        for cte in &with.cte_tables {
            count += count_subqueries_in_set_expr(&cte.query.body);
            if let Some(with2) = &cte.query.with {
                count += with2.cte_tables.len();
            }
        }
    }

    // Count subqueries in the main query body.
    count += count_subqueries_in_set_expr(&query.body);

    count
}

fn count_subqueries_in_set_expr(expr: &SetExpr) -> usize {
    match expr {
        SetExpr::Select(sel) => count_subqueries_in_select(sel),
        SetExpr::Query(inner) => count_subqueries_in_query(inner),
        SetExpr::SetOperation { left, right, .. } => {
            count_subqueries_in_set_expr(left) + count_subqueries_in_set_expr(right)
        }
        _ => 0,
    }
}

fn count_subqueries_in_select(sel: &Select) -> usize {
    let mut count = 0;

    // Projection (SELECT list).
    for item in &sel.projection {
        let expr = match item {
            SelectItem::UnnamedExpr(e) => Some(e),
            SelectItem::ExprWithAlias { expr, .. } => Some(expr),
            _ => None,
        };
        if let Some(e) = expr {
            count += count_subqueries_in_expr(e);
        }
    }

    // FROM clause (derived tables / subqueries).
    for twj in &sel.from {
        count += count_subqueries_in_table_factor(&twj.relation);
        for join in &twj.joins {
            count += count_subqueries_in_table_factor(&join.relation);
        }
    }

    // WHERE clause.
    if let Some(selection) = &sel.selection {
        count += count_subqueries_in_expr(selection);
    }

    // HAVING clause.
    if let Some(having) = &sel.having {
        count += count_subqueries_in_expr(having);
    }

    count
}

fn count_subqueries_in_table_factor(tf: &TableFactor) -> usize {
    if let TableFactor::Derived { subquery, .. } = tf {
        // A derived table (subquery in FROM) counts as one subquery plus any
        // subqueries nested within it.
        1 + count_subqueries_in_query(subquery)
    } else {
        0
    }
}

/// Recursively count all subquery expressions within an expression tree.
/// Counted variants: `Subquery`, `InSubquery`, `Exists`.
fn count_subqueries_in_expr(expr: &Expr) -> usize {
    match expr {
        Expr::Subquery(q) => {
            // The subquery itself counts as 1; recurse into it to count nested ones.
            1 + count_subqueries_in_query(q)
        }
        Expr::InSubquery { subquery, expr: e, .. } => {
            1 + count_subqueries_in_query(subquery) + count_subqueries_in_expr(e)
        }
        Expr::Exists { subquery, .. } => {
            1 + count_subqueries_in_query(subquery)
        }
        Expr::BinaryOp { left, right, .. } => {
            count_subqueries_in_expr(left) + count_subqueries_in_expr(right)
        }
        Expr::UnaryOp { expr: inner, .. } => count_subqueries_in_expr(inner),
        Expr::Nested(inner) => count_subqueries_in_expr(inner),
        Expr::Between { expr: e, low, high, .. } => {
            count_subqueries_in_expr(e)
                + count_subqueries_in_expr(low)
                + count_subqueries_in_expr(high)
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            operand.as_ref().map_or(0, |e| count_subqueries_in_expr(e))
                + conditions.iter().map(|e| count_subqueries_in_expr(e)).sum::<usize>()
                + results.iter().map(|e| count_subqueries_in_expr(e)).sum::<usize>()
                + else_result
                    .as_ref()
                    .map_or(0, |e| count_subqueries_in_expr(e))
        }
        _ => 0,
    }
}

// ── keyword position helpers ──────────────────────────────────────────────────

/// Find the `nth` (0-indexed) occurrence of a keyword (case-insensitive,
/// word-boundary, outside strings/comments) in `source`. Returns a 1-indexed
/// (line, col) pair. Falls back to (1, 1) if not found.
fn find_nth_keyword_pos(source: &str, keyword: &str, nth: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);
    let kw_upper: Vec<u8> = keyword.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let kw_len = kw_upper.len();

    let mut count = 0;
    let mut i = 0;
    while i + kw_len <= len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }

        // Word boundary before.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive match.
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(kw_upper.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if matches {
            // Word boundary after.
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);
            let all_code = (i..i + kw_len).all(|k| skip_map.is_code(k));

            if after_ok && all_code {
                if count == nth {
                    return line_col(source, i);
                }
                count += 1;
            }
        }

        i += 1;
    }

    (1, 1)
}

/// Find the position of the `nth` (0-indexed) SELECT keyword.
/// The outer query is SELECT #0; the first subquery is SELECT #1.
fn find_nth_select_pos(source: &str, nth: usize) -> (usize, usize) {
    find_nth_keyword_pos(source, "SELECT", nth)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
