use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Join, JoinConstraint, JoinOperator, Query, SetExpr, Statement, TableFactor};

pub struct NaturalJoin;

impl Rule for NaturalJoin {
    fn name(&self) -> &'static str {
        "NaturalJoin"
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

// ── AST walking ───────────────────────────────────────────────────────────────

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
                for join in &twj.joins {
                    check_join(join, source, rule, diags);
                }
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

/// Returns true if the `JoinOperator` uses `JoinConstraint::Natural`.
///
/// In sqlparser 0.53, NATURAL JOIN is not a standalone `JoinOperator` variant.
/// Instead it is expressed as any directional join variant with a `Natural`
/// constraint, e.g. `Inner(JoinConstraint::Natural)` for a plain
/// `NATURAL JOIN`, or `LeftOuter(JoinConstraint::Natural)` for
/// `NATURAL LEFT JOIN`, etc.
fn is_natural(op: &JoinOperator) -> bool {
    match op {
        JoinOperator::Inner(c) => matches!(c, JoinConstraint::Natural),
        JoinOperator::LeftOuter(c) => matches!(c, JoinConstraint::Natural),
        JoinOperator::RightOuter(c) => matches!(c, JoinConstraint::Natural),
        JoinOperator::FullOuter(c) => matches!(c, JoinConstraint::Natural),
        _ => false,
    }
}

fn check_join(join: &Join, source: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if is_natural(&join.join_operator) {
        let (line, col) = find_keyword_pos(source, "NATURAL");
        diags.push(Diagnostic {
            rule,
            message: "NATURAL JOIN depends on column naming conventions; use explicit JOIN ON instead".to_string(),
            line,
            col,
        });
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

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the first occurrence of a keyword (case-insensitive, word-boundary)
/// in `source`. Returns a 1-indexed (line, col) pair. Falls back to (1, 1) if
/// not found.
fn find_keyword_pos(source: &str, keyword: &str) -> (usize, usize) {
    let upper = source.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let kw_len = kw_upper.len();
    let bytes = upper.as_bytes();
    let len = bytes.len();

    let mut pos = 0;
    while pos + kw_len <= len {
        if let Some(rel) = upper[pos..].find(kw_upper.as_str()) {
            let abs = pos + rel;

            // Word boundary check.
            let before_ok = abs == 0 || {
                let b = bytes[abs - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            let after = abs + kw_len;
            let after_ok = after >= len || {
                let b = bytes[after];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if before_ok && after_ok {
                return line_col(source, abs);
            }

            pos = abs + 1;
        } else {
            break;
        }
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
