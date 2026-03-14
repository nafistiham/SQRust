use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Offset, Query, Select, SetExpr, Statement, TableFactor, Value};

use crate::capitalisation::{is_word_char, SkipMap};

/// Threshold above which an OFFSET is considered large (exclusive).
const LARGE_OFFSET_THRESHOLD: i64 = 1000;

pub struct LargeOffset;

impl Rule for LargeOffset {
    fn name(&self) -> &'static str {
        "Structure/LargeOffset"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let mut offset_counter: usize = 0;

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, ctx, &mut offset_counter, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    ctx: &FileContext,
    offset_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs first.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, offset_counter, diags);
        }
    }

    // Check this query's own OFFSET.
    if let Some(offset) = &query.offset {
        if let Some(n) = extract_large_offset(offset) {
            let occurrence = *offset_counter;
            *offset_counter += 1;
            let (line, col) = find_nth_keyword_pos(&ctx.source, "OFFSET", occurrence);
            diags.push(Diagnostic {
                rule: "Structure/LargeOffset",
                message: format!(
                    "OFFSET {n} forces a full scan of {n} rows — consider cursor-based or keyset pagination for large offsets"
                ),
                line,
                col,
            });
        }
    }

    // Recurse into the body (subqueries in FROM, WHERE, SELECT).
    check_set_expr(&query.body, ctx, offset_counter, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    ctx: &FileContext,
    offset_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, ctx, offset_counter, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, ctx, offset_counter, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, offset_counter, diags);
            check_set_expr(right, ctx, offset_counter, diags);
        }
        _ => {}
    }
}

fn check_select(
    select: &Select,
    ctx: &FileContext,
    offset_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // FROM clause — check Derived (subquery) tables.
    for table_with_joins in &select.from {
        check_table_factor(&table_with_joins.relation, ctx, offset_counter, diags);
        for join in &table_with_joins.joins {
            check_table_factor(&join.relation, ctx, offset_counter, diags);
        }
    }
}

fn check_table_factor(
    factor: &TableFactor,
    ctx: &FileContext,
    offset_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        check_query(subquery, ctx, offset_counter, diags);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Returns `Some(n)` if `offset` is a numeric literal greater than the threshold.
fn extract_large_offset(offset: &Offset) -> Option<i64> {
    if let Expr::Value(Value::Number(s, _)) = &offset.value {
        let n = s.parse::<i64>().unwrap_or(0);
        if n > LARGE_OFFSET_THRESHOLD {
            return Some(n);
        }
    }
    None
}

/// Finds the `nth` (0-indexed) occurrence of `keyword` (case-insensitive,
/// word-boundary) in `source`. Returns a 1-indexed (line, col) pair.
/// Falls back to (1, 1) if not found.
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
