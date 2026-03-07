use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, GroupByExpr, Query, SetExpr, Statement, TableFactor};

pub struct SubqueryInGroupBy;

impl Rule for SubqueryInGroupBy {
    fn name(&self) -> &'static str {
        "Ambiguous/SubqueryInGroupBy"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            collect_from_statement(stmt, ctx, &mut diags);
        }
        diags
    }
}

fn collect_from_statement(stmt: &Statement, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Statement::Query(query) = stmt {
        collect_from_query(query, ctx, diags);
    }
}

fn collect_from_query(query: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, ctx, diags);
        }
    }
    collect_from_set_expr(&query.body, ctx, diags);
}

fn collect_from_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(select) => {
            // Check GROUP BY expressions for subqueries.
            if let GroupByExpr::Expressions(exprs, _) = &select.group_by {
                for group_expr in exprs {
                    if contains_subquery(group_expr) {
                        let (line, col) = find_keyword_position(&ctx.source, "GROUP BY")
                            .unwrap_or((1, 1));
                        diags.push(Diagnostic {
                            rule: "Ambiguous/SubqueryInGroupBy",
                            message: "Subquery in GROUP BY is non-standard and unsupported by most databases"
                                .to_string(),
                            line,
                            col,
                        });
                    }
                }
            }
            // GroupByExpr::All(_) => no flag — GROUP BY ALL is not a subquery.

            // Recurse into FROM subqueries so we catch violations in CTEs/derived tables.
            for twj in &select.from {
                collect_from_table_factor(&twj.relation, ctx, diags);
                for join in &twj.joins {
                    collect_from_table_factor(&join.relation, ctx, diags);
                }
            }
        }
        SetExpr::Query(inner) => {
            collect_from_query(inner, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            collect_from_set_expr(left, ctx, diags);
            collect_from_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}

fn collect_from_table_factor(
    factor: &TableFactor,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        collect_from_query(subquery, ctx, diags);
    }
}

/// Returns `true` if `expr` is or contains a subquery (`Subquery`, `InSubquery`,
/// or `Exists`). Recurses into `BinaryOp` and `Nested` to catch subqueries
/// embedded inside larger expressions.
fn contains_subquery(expr: &Expr) -> bool {
    match expr {
        Expr::Subquery(_) | Expr::Exists { .. } | Expr::InSubquery { .. } => true,
        Expr::BinaryOp { left, right, .. } => {
            contains_subquery(left) || contains_subquery(right)
        }
        Expr::Nested(inner) => contains_subquery(inner),
        Expr::UnaryOp { expr: inner, .. } => contains_subquery(inner),
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            operand.as_deref().is_some_and(contains_subquery)
                || conditions.iter().any(contains_subquery)
                || results.iter().any(contains_subquery)
                || else_result.as_deref().is_some_and(contains_subquery)
        }
        _ => false,
    }
}

/// Finds the first occurrence of the two-word phrase `GROUP BY` (case-insensitive,
/// with any whitespace between) in `source` and returns `Some((line, col))`.
/// Falls back to `None` if not found.
fn find_keyword_position(source: &str, _keyword: &str) -> Option<(usize, usize)> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let group = b"GROUP";
    let by = b"BY";

    let mut i = 0;
    while i < len {
        // Try to match GROUP at a word boundary.
        if i + group.len() <= len
            && bytes[i..i + group.len()].eq_ignore_ascii_case(group)
            && (i == 0 || !is_word_char(bytes[i - 1]))
            && (i + group.len() >= len || !is_word_char(bytes[i + group.len()]))
        {
            // Skip whitespace after GROUP.
            let mut j = i + group.len();
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                j += 1;
            }
            // Try to match BY at a word boundary.
            if j + by.len() <= len
                && bytes[j..j + by.len()].eq_ignore_ascii_case(by)
                && (j + by.len() >= len || !is_word_char(bytes[j + by.len()]))
            {
                return Some(offset_to_line_col(source, i));
            }
        }
        i += 1;
    }
    None
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Converts a byte offset to 1-indexed (line, col).
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
