use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{BinaryOperator, Expr, Query, Select, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct HavingConditionsCount {
    /// Maximum number of conditions allowed in a HAVING clause.
    /// A clause with N conditions is connected by N-1 AND/OR operators.
    /// When the condition count exceeds this maximum the clause is flagged.
    pub max_conditions: usize,
}

impl Default for HavingConditionsCount {
    fn default() -> Self {
        HavingConditionsCount { max_conditions: 5 }
    }
}

impl Rule for HavingConditionsCount {
    fn name(&self) -> &'static str {
        "Structure/HavingConditionsCount"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.max_conditions, ctx, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    max: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, max, ctx, diags);
        }
    }

    check_set_expr(&query.body, max, ctx, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    max: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, max, ctx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, max, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, max, ctx, diags);
            check_set_expr(right, max, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    max: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(having) = &sel.having {
        // AND/OR operator count; number of distinct conditions = ops + 1
        let ops = count_and_or_ops(having);
        let conditions = ops + 1;
        if conditions > max {
            let (line, col) = find_keyword_pos(&ctx.source, "HAVING");
            diags.push(Diagnostic {
                rule: "Structure/HavingConditionsCount",
                message: format!(
                    "HAVING clause has {conditions} conditions, exceeding the maximum of {max}",
                ),
                line,
                col,
            });
        }
    }

    // Recurse into subqueries in the FROM clause.
    for twj in &sel.from {
        check_table_factor(&twj.relation, max, ctx, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, max, ctx, diags);
        }
    }
}

fn check_table_factor(
    tf: &TableFactor,
    max: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, max, ctx, diags);
    }
}

// ── condition counting ────────────────────────────────────────────────────────

/// Count the number of top-level AND/OR binary operations in an expression.
/// Each AND or OR operator adds 1 to the count.
/// A HAVING clause with N conditions is connected by N-1 such operators.
fn count_and_or_ops(expr: &Expr) -> usize {
    match expr {
        Expr::BinaryOp {
            left,
            op: BinaryOperator::And | BinaryOperator::Or,
            right,
        } => 1 + count_and_or_ops(left) + count_and_or_ops(right),
        Expr::BinaryOp { left, right, .. } => {
            count_and_or_ops(left) + count_and_or_ops(right)
        }
        Expr::UnaryOp { expr: inner, .. } => count_and_or_ops(inner),
        Expr::Nested(inner) => count_and_or_ops(inner),
        _ => 0,
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
