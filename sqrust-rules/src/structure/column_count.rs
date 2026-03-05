use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct ColumnCount {
    /// Maximum number of columns allowed in a single SELECT list.
    pub max_columns: usize,
}

impl Default for ColumnCount {
    fn default() -> Self {
        ColumnCount { max_columns: 20 }
    }
}

impl Rule for ColumnCount {
    fn name(&self) -> &'static str {
        "Structure/ColumnCount"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        // Collect all SELECT keyword offsets in source order so we can assign
        // accurate line/col to each violating SELECT.
        let select_offsets = collect_select_offsets(&ctx.source);

        let mut diags = Vec::new();
        // `select_index` tracks how many SELECT keywords we have consumed so
        // far as we walk the AST in source order.
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
    // CTEs are visited before the main body in source order.
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
    // Each SELECT in the AST corresponds to one SELECT keyword in source.
    let offset = offsets.get(*idx).copied().unwrap_or(0);
    *idx += 1;

    let count = sel.projection.len();
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

        // Word boundary check before.
        let before_ok = i == 0 || {
            let b = bytes[i - 1];
            !is_word_char(b)
        };
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
            // Word boundary check after.
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);

            // All bytes of SELECT must be real code.
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
