use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Distinct, Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct SelectDistinctStar;

impl Rule for SelectDistinctStar {
    fn name(&self) -> &'static str {
        "SelectDistinctStar"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        // Collect all byte offsets of `DISTINCT` keywords in source order.
        let distinct_offsets = collect_distinct_offsets(&ctx.source);
        let mut distinct_index: usize = 0;
        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(
                    query,
                    self.name(),
                    &ctx.source,
                    &distinct_offsets,
                    &mut distinct_index,
                    &mut diags,
                );
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, source, offsets, idx, diags);
        }
    }
    check_set_expr(&query.body, rule, source, offsets, idx, diags);
}

fn check_set_expr(
    body: &SetExpr,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match body {
        SetExpr::Select(sel) => check_select(sel, rule, source, offsets, idx, diags),
        SetExpr::Query(q) => check_query(q, rule, source, offsets, idx, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, rule, source, offsets, idx, diags);
            check_set_expr(right, rule, source, offsets, idx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    let has_distinct = matches!(&sel.distinct, Some(Distinct::Distinct));

    // Check if the projection is a single wildcard (bare * or qualified t.*)
    let is_star = sel.projection.len() == 1
        && matches!(
            &sel.projection[0],
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
        );

    if has_distinct && is_star {
        let offset = offsets.get(*idx).copied().unwrap_or(0);
        let (line, col) = line_col(source, offset);
        diags.push(Diagnostic {
            rule,
            message: "SELECT DISTINCT * is redundant; specify columns explicitly".to_string(),
            line,
            col,
        });
    }

    // Consume the DISTINCT offset slot regardless of whether we flagged,
    // so subsequent selects get the correct offset.
    if has_distinct {
        *idx += 1;
    }

    // Recurse into subqueries in the FROM clause.
    for table in &sel.from {
        recurse_table_factor(&table.relation, rule, source, offsets, idx, diags);
        for join in &table.joins {
            recurse_table_factor(&join.relation, rule, source, offsets, idx, diags);
        }
    }

    // Recurse into subqueries in the projection (e.g. scalar subqueries).
    for item in &sel.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr_for_subqueries(e, rule, source, offsets, idx, diags);
        }
    }

    // Recurse into subqueries in WHERE.
    if let Some(selection) = &sel.selection {
        check_expr_for_subqueries(selection, rule, source, offsets, idx, diags);
    }

    // Recurse into subqueries in HAVING.
    if let Some(having) = &sel.having {
        check_expr_for_subqueries(having, rule, source, offsets, idx, diags);
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, rule, source, offsets, idx, diags);
    }
}

/// Walk expressions only for nested subqueries (scalar subqueries in SELECT
/// list, WHERE, HAVING).  This does NOT touch InList — only Subquery nodes.
fn check_expr_for_subqueries(
    expr: &Expr,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Subquery(q) => check_query(q, rule, source, offsets, idx, diags),
        Expr::BinaryOp { left, right, .. } => {
            check_expr_for_subqueries(left, rule, source, offsets, idx, diags);
            check_expr_for_subqueries(right, rule, source, offsets, idx, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr_for_subqueries(inner, rule, source, offsets, idx, diags);
        }
        Expr::Nested(inner) => {
            check_expr_for_subqueries(inner, rule, source, offsets, idx, diags);
        }
        Expr::InSubquery { subquery, .. } => {
            check_query(subquery, rule, source, offsets, idx, diags);
        }
        Expr::Exists { subquery, .. } => {
            check_query(subquery, rule, source, offsets, idx, diags);
        }
        _ => {}
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Collect byte offsets of every `DISTINCT` keyword (case-insensitive,
/// word-boundary, outside strings/comments) in source order.
fn collect_distinct_offsets(source: &str) -> Vec<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);
    let kw = b"DISTINCT";
    let kw_len = kw.len();
    let mut offsets = Vec::new();

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
            .zip(kw.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if matches {
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);
            let all_code = (i..i + kw_len).all(|k| skip_map.is_code(k));

            if after_ok && all_code {
                offsets.push(i);
                i += kw_len;
                continue;
            }
        }

        i += 1;
    }

    offsets
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
