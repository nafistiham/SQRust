use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{BinaryOperator, Expr, Join, JoinConstraint, JoinOperator, Query, SetExpr,
    Statement, TableFactor};

pub struct OrInJoinCondition;

impl Rule for OrInJoinCondition {
    fn name(&self) -> &'static str {
        "Ambiguous/OrInJoinCondition"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &ctx.source, &mut diags);
            }
        }
        diags
    }
}

fn check_query(query: &Query, source: &str, diags: &mut Vec<Diagnostic>) {
    // Recurse into CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, diags);
        }
    }
    check_set_expr(&query.body, source, diags);
}

fn check_set_expr(expr: &SetExpr, source: &str, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(select) => {
            for twj in &select.from {
                // Check each join in this FROM item.
                for join in &twj.joins {
                    check_join(join, source, diags);
                }
                // Recurse into subqueries inside table factors.
                recurse_table_factor(&twj.relation, source, diags);
                for join in &twj.joins {
                    recurse_table_factor(&join.relation, source, diags);
                }
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, diags);
            check_set_expr(right, source, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, source, diags);
        }
        _ => {}
    }
}

/// Extracts the `ON` expression from a join operator, if it has one.
fn on_expr(join: &Join) -> Option<&Expr> {
    match &join.join_operator {
        JoinOperator::Inner(JoinConstraint::On(e))
        | JoinOperator::LeftOuter(JoinConstraint::On(e))
        | JoinOperator::RightOuter(JoinConstraint::On(e))
        | JoinOperator::FullOuter(JoinConstraint::On(e)) => Some(e),
        _ => None,
    }
}

/// Checks a single join for an OR condition in its ON clause.
fn check_join(join: &Join, source: &str, diags: &mut Vec<Diagnostic>) {
    if let Some(expr) = on_expr(join) {
        if has_or(expr) {
            let (line, col) = find_or_position(source);
            diags.push(Diagnostic {
                rule: "Ambiguous/OrInJoinCondition",
                message: "OR condition in JOIN ON clause; this may produce unintended cross-join-like results"
                    .to_string(),
                line,
                col,
            });
        }
    }
}

/// Returns `true` if `expr` contains an OR operator at any nesting level.
fn has_or(expr: &Expr) -> bool {
    match expr {
        Expr::BinaryOp {
            op: BinaryOperator::Or,
            ..
        } => true,
        Expr::BinaryOp { left, right, .. } => has_or(left) || has_or(right),
        Expr::Nested(e) => has_or(e),
        Expr::UnaryOp { expr: e, .. } => has_or(e),
        _ => false,
    }
}

/// Recurses into a `TableFactor::Derived` (subquery) to check joins inside it.
fn recurse_table_factor(tf: &TableFactor, source: &str, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, diags);
    }
}

/// Finds the first word-boundary occurrence of `OR` (case-insensitive) in
/// `source` and returns a 1-indexed (line, col). Falls back to (1, 1).
fn find_or_position(source: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let kw = b"OR";
    let kw_len = kw.len();

    let mut i = 0;
    while i + kw_len <= len {
        let matches = bytes[i].eq_ignore_ascii_case(&kw[0])
            && bytes[i + 1].eq_ignore_ascii_case(&kw[1]);
        if matches {
            let before_ok =
                i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let after = i + kw_len;
            let after_ok = after >= len
                || (!bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_');
            if before_ok && after_ok {
                return offset_to_line_col(source, i);
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
