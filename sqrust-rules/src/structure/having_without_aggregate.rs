use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct HavingWithoutAggregate;

impl Rule for HavingWithoutAggregate {
    fn name(&self) -> &'static str {
        "HavingWithoutAggregate"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.name(), ctx, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, ctx, diags);
        }
    }
    check_set_expr(&query.body, rule, ctx, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, rule, ctx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, rule, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, rule, ctx, diags);
            check_set_expr(right, rule, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(having) = &sel.having {
        if !has_aggregate(having) {
            let (line, col) = find_keyword_pos(&ctx.source, "HAVING");
            diags.push(Diagnostic {
                rule,
                message: "HAVING clause contains no aggregate function; use WHERE instead"
                    .to_string(),
                line,
                col,
            });
        }
    }

    // Recurse into subqueries in the FROM clause.
    for table_with_joins in &sel.from {
        recurse_table_factor(&table_with_joins.relation, rule, ctx, diags);
        for join in &table_with_joins.joins {
            recurse_table_factor(&join.relation, rule, ctx, diags);
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, rule, ctx, diags);
    }
}

// ── aggregate detection ───────────────────────────────────────────────────────

fn has_aggregate(expr: &Expr) -> bool {
    match expr {
        Expr::Function(func) => {
            let name = func
                .name
                .0
                .last()
                .map(|i| i.value.to_uppercase())
                .unwrap_or_default();
            matches!(
                name.as_str(),
                "COUNT"
                    | "SUM"
                    | "AVG"
                    | "MIN"
                    | "MAX"
                    | "ARRAY_AGG"
                    | "STRING_AGG"
                    | "GROUP_CONCAT"
                    | "STDDEV"
                    | "VARIANCE"
                    | "MEDIAN"
                    | "LISTAGG"
                    | "FIRST_VALUE"
                    | "LAST_VALUE"
            )
        }
        Expr::BinaryOp { left, right, .. } => has_aggregate(left) || has_aggregate(right),
        Expr::UnaryOp { expr, .. } => has_aggregate(expr),
        Expr::Nested(e) => has_aggregate(e),
        Expr::Between {
            expr, low, high, ..
        } => has_aggregate(expr) || has_aggregate(low) || has_aggregate(high),
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            operand.as_ref().map_or(false, |e| has_aggregate(e))
                || conditions.iter().any(|e| has_aggregate(e))
                || results.iter().any(|e| has_aggregate(e))
                || else_result.as_ref().map_or(false, |e| has_aggregate(e))
        }
        _ => false,
    }
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the first occurrence of a keyword (case-insensitive, word-boundary,
/// outside strings/comments) in `source`. Returns a 1-indexed (line, col)
/// pair. Falls back to (1, 1) if not found.
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
