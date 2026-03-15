use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct MaxSelectColumns {
    /// Maximum number of non-wildcard columns allowed in a single SELECT list.
    pub max_columns: usize,
}

impl Default for MaxSelectColumns {
    fn default() -> Self {
        MaxSelectColumns { max_columns: 20 }
    }
}

impl Rule for MaxSelectColumns {
    fn name(&self) -> &'static str {
        "Structure/MaxSelectColumns"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let select_offsets = collect_select_offsets(&ctx.source);

        let mut diags = Vec::new();
        let mut select_index: usize = 0;

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(
                    query,
                    self.max_columns,
                    self.name(),
                    &ctx.source,
                    &select_offsets,
                    &mut select_index,
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
    max: usize,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, max, rule, source, offsets, idx, diags);
        }
    }
    check_set_expr(&query.body, max, rule, source, offsets, idx, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    max: usize,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, max, rule, source, offsets, idx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, max, rule, source, offsets, idx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, max, rule, source, offsets, idx, diags);
            check_set_expr(right, max, rule, source, offsets, idx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    max: usize,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    let offset = offsets.get(*idx).copied().unwrap_or(0);
    *idx += 1;

    // Count only non-wildcard items.
    let count = sel.projection.iter().filter(|item| {
        !matches!(item, SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(..))
    }).count();

    if count > max {
        let (line, col) = line_col(source, offset);
        diags.push(Diagnostic {
            rule,
            message: format!(
                "SELECT has {count} columns; maximum is {max}",
                count = count,
                max = max,
            ),
            line,
            col,
        });
    }

    // Recurse into subqueries inside FROM / JOIN clauses.
    for table in &sel.from {
        recurse_table_factor(&table.relation, max, rule, source, offsets, idx, diags);
        for join in &table.joins {
            recurse_table_factor(&join.relation, max, rule, source, offsets, idx, diags);
        }
    }

    // Recurse into subqueries inside the WHERE clause.
    if let Some(selection) = &sel.selection {
        recurse_expr(selection, max, rule, source, offsets, idx, diags);
    }

    // Recurse into scalar subqueries in the projection list.
    for item in &sel.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            recurse_expr(e, max, rule, source, offsets, idx, diags);
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    max: usize,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, max, rule, source, offsets, idx, diags);
    }
}

fn recurse_expr(
    expr: &Expr,
    max: usize,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Subquery(q) => check_query(q, max, rule, source, offsets, idx, diags),
        Expr::InSubquery { subquery, .. } => {
            check_query(subquery, max, rule, source, offsets, idx, diags)
        }
        Expr::Exists { subquery, .. } => {
            check_query(subquery, max, rule, source, offsets, idx, diags)
        }
        Expr::BinaryOp { left, right, .. } => {
            recurse_expr(left, max, rule, source, offsets, idx, diags);
            recurse_expr(right, max, rule, source, offsets, idx, diags);
        }
        _ => {}
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Collect byte offsets of every `SELECT` keyword (case-insensitive,
/// word-boundary, outside strings/comments) in source order.
fn collect_select_offsets(source: &str) -> Vec<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);
    let kw = b"SELECT";
    let kw_len = kw.len();
    let mut offsets = Vec::new();

    let mut i = 0;
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
