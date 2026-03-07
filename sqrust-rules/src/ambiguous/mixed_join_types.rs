use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{JoinOperator, Query, SetExpr, Statement, TableFactor};

pub struct MixedJoinTypes;

impl Rule for MixedJoinTypes {
    fn name(&self) -> &'static str {
        "Ambiguous/MixedJoinTypes"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &ctx.source, self.name(), &mut diags);
            }
        }
        diags
    }
}

fn check_query(query: &Query, source: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    // Recurse into CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, rule, diags);
        }
    }
    check_set_expr(&query.body, source, rule, diags);
}

fn check_set_expr(expr: &SetExpr, source: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            for twj in &sel.from {
                // Classify each join in this FROM clause.
                let mut has_inner = false;
                let mut has_outer = false;

                for join in &twj.joins {
                    match &join.join_operator {
                        JoinOperator::Inner(_) | JoinOperator::CrossJoin => {
                            has_inner = true;
                        }
                        JoinOperator::LeftOuter(_)
                        | JoinOperator::RightOuter(_)
                        | JoinOperator::FullOuter(_) => {
                            has_outer = true;
                        }
                        _ => {}
                    }
                }

                if has_inner && has_outer {
                    let (line, col) = find_keyword_position(source, "FROM");
                    diags.push(Diagnostic {
                        rule,
                        message: "Mixing INNER JOIN and LEFT/RIGHT/FULL JOIN in the same FROM clause may produce ambiguous results".to_string(),
                        line,
                        col,
                    });
                }

                // Recurse into subqueries in the base relation and all joins.
                recurse_table_factor(&twj.relation, source, rule, diags);
                for join in &twj.joins {
                    recurse_table_factor(&join.relation, source, rule, diags);
                }
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, rule, diags);
            check_set_expr(right, source, rule, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, source, rule, diags);
        }
        _ => {}
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    source: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, rule, diags);
    }
}

/// Finds the first occurrence of `keyword` (case-insensitive, word-boundary-checked)
/// in `source` and returns a 1-indexed (line, col). Falls back to (1, 1) if not found.
fn find_keyword_position(source: &str, keyword: &str) -> (usize, usize) {
    let upper = source.to_ascii_uppercase();
    let kw_upper = keyword.to_ascii_uppercase();
    let bytes = upper.as_bytes();
    let kw_bytes = kw_upper.as_bytes();
    let kw_len = kw_bytes.len();

    let mut i = 0;
    while i + kw_len <= bytes.len() {
        if bytes[i..i + kw_len] == *kw_bytes {
            let before_ok =
                i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let after = i + kw_len;
            let after_ok = after >= bytes.len()
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
