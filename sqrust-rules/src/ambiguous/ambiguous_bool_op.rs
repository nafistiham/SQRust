use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    BinaryOperator, Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
};

pub struct AmbiguousBoolOp;

impl Rule for AmbiguousBoolOp {
    fn name(&self) -> &'static str {
        "Ambiguous/AmbiguousBoolOp"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let mut or_occurrence = 0usize;
        let mut and_occurrence = 0usize;

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(
                    query,
                    &ctx.source,
                    &mut or_occurrence,
                    &mut and_occurrence,
                    &mut diags,
                );
            }
        }
        diags
    }
}

fn check_query(
    query: &Query,
    source: &str,
    or_occ: &mut usize,
    and_occ: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, or_occ, and_occ, diags);
        }
    }
    check_set_expr(&query.body, source, or_occ, and_occ, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    source: &str,
    or_occ: &mut usize,
    and_occ: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, source, or_occ, and_occ, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, or_occ, and_occ, diags);
            check_set_expr(right, source, or_occ, and_occ, diags);
        }
        SetExpr::Query(inner) => check_query(inner, source, or_occ, and_occ, diags),
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    source: &str,
    or_occ: &mut usize,
    and_occ: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // Check WHERE clause
    if let Some(selection) = &sel.selection {
        check_expr(selection, source, or_occ, and_occ, diags);
    }

    // Check HAVING clause
    if let Some(having) = &sel.having {
        check_expr(having, source, or_occ, and_occ, diags);
    }

    // Check SELECT projection for scalar subqueries
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                check_expr(e, source, or_occ, and_occ, diags);
            }
            _ => {}
        }
    }

    // Check FROM subqueries
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, source, or_occ, and_occ, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, source, or_occ, and_occ, diags);
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    source: &str,
    or_occ: &mut usize,
    and_occ: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, or_occ, and_occ, diags);
    }
}

fn check_expr(
    expr: &Expr,
    source: &str,
    or_occ: &mut usize,
    and_occ: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::BinaryOp {
            left,
            op: BinaryOperator::Or,
            right,
        } => {
            // Flag if either child is a raw (unwrapped) AND expression.
            let left_is_raw_and =
                matches!(left.as_ref(), Expr::BinaryOp { op: BinaryOperator::And, .. });
            let right_is_raw_and =
                matches!(right.as_ref(), Expr::BinaryOp { op: BinaryOperator::And, .. });

            if left_is_raw_and || right_is_raw_and {
                let (line, col) = find_nth_keyword_position(source, "OR", *or_occ);
                *or_occ += 1;
                diags.push(Diagnostic {
                    rule: "Ambiguous/AmbiguousBoolOp",
                    message: "AND and OR mixed without parentheses; add parentheses to clarify precedence".to_string(),
                    line,
                    col,
                });
            }

            // Recurse into children (do NOT recurse into Nested — it's already safe).
            check_expr(left, source, or_occ, and_occ, diags);
            check_expr(right, source, or_occ, and_occ, diags);
        }

        Expr::BinaryOp {
            left,
            op: BinaryOperator::And,
            right,
        } => {
            // Flag if either child is a raw (unwrapped) OR expression.
            let left_is_raw_or =
                matches!(left.as_ref(), Expr::BinaryOp { op: BinaryOperator::Or, .. });
            let right_is_raw_or =
                matches!(right.as_ref(), Expr::BinaryOp { op: BinaryOperator::Or, .. });

            if left_is_raw_or || right_is_raw_or {
                let (line, col) = find_nth_keyword_position(source, "AND", *and_occ);
                *and_occ += 1;
                diags.push(Diagnostic {
                    rule: "Ambiguous/AmbiguousBoolOp",
                    message: "AND and OR mixed without parentheses; add parentheses to clarify precedence".to_string(),
                    line,
                    col,
                });
            }

            check_expr(left, source, or_occ, and_occ, diags);
            check_expr(right, source, or_occ, and_occ, diags);
        }

        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, source, or_occ, and_occ, diags);
            check_expr(right, source, or_occ, and_occ, diags);
        }

        Expr::Nested(inner) => {
            // Inside Nested, the parens make things explicit — still recurse,
            // but the outer rule already doesn't flag across Nested boundaries.
            check_expr(inner, source, or_occ, and_occ, diags);
        }

        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, source, or_occ, and_occ, diags);
        }

        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            check_query(q, source, or_occ, and_occ, diags);
        }

        _ => {}
    }
}

/// Finds the `n`-th (0-indexed) word-boundary occurrence of `keyword`
/// (case-insensitive) in `source`. Returns 1-indexed (line, col).
/// Falls back to (1, 1) if not found.
fn find_nth_keyword_position(source: &str, keyword: &str, n: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let kw = keyword.as_bytes();
    let kw_len = kw.len();

    let mut found = 0usize;
    let mut i = 0;
    while i + kw_len <= len {
        // Case-insensitive match
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(kw.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if matches {
            let before_ok =
                i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let after = i + kw_len;
            let after_ok =
                after >= len || (!bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_');

            if before_ok && after_ok {
                if found == n {
                    return offset_to_line_col(source, i);
                }
                found += 1;
                i += kw_len;
                continue;
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
