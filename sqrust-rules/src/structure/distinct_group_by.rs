use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{GroupByExpr, Query, Select, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct DistinctGroupBy;

impl Rule for DistinctGroupBy {
    fn name(&self) -> &'static str {
        "Structure/DistinctGroupBy"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let distinct_offsets = collect_distinct_offsets(&ctx.source);
        let mut diags = Vec::new();
        let mut distinct_index: usize = 0;

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
    // Visit CTEs in source order first.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, source, offsets, idx, diags);
        }
    }
    check_set_expr(&query.body, rule, source, offsets, idx, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, rule, source, offsets, idx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, rule, source, offsets, idx, diags);
        }
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
    let has_distinct = sel.distinct.is_some();
    let has_group_by = matches!(&sel.group_by, GroupByExpr::Expressions(exprs, _) if !exprs.is_empty());

    if has_distinct && has_group_by {
        // Use the next DISTINCT keyword position in source order.
        let offset = offsets.get(*idx).copied().unwrap_or(0);
        let (line, col) = line_col(source, offset);
        diags.push(Diagnostic {
            rule,
            message: "SELECT DISTINCT with GROUP BY is redundant; GROUP BY already deduplicates rows".to_string(),
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

        // Word boundary check before.
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
            // Word boundary check after.
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
