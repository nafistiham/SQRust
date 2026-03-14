use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SetExpr, Statement, TableFactor};

pub struct UnionBranchLimit;

impl Rule for UnionBranchLimit {
    fn name(&self) -> &'static str {
        "Structure/UnionBranchLimit"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                collect_from_query(query, self.name(), &ctx.source, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn collect_from_query(
    query: &Query,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    // Recurse into CTEs first.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, rule, source, diags);
        }
    }
    collect_from_set_expr(&query.body, rule, source, diags);
}

fn collect_from_set_expr(
    expr: &SetExpr,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::SetOperation { left, right, .. } => {
            // Check each branch: a branch is flagged when it is a Query node
            // (i.e. a parenthesized subquery) that carries its own LIMIT or FETCH.
            check_branch(left, rule, source, diags);
            check_branch(right, rule, source, diags);

            // Recurse into both branches for nested set operations.
            collect_from_set_expr(left, rule, source, diags);
            collect_from_set_expr(right, rule, source, diags);
        }
        SetExpr::Select(sel) => {
            // Recurse into subqueries in FROM so we catch violations inside
            // derived tables.
            for twj in &sel.from {
                recurse_table_factor(&twj.relation, rule, source, diags);
                for join in &twj.joins {
                    recurse_table_factor(&join.relation, rule, source, diags);
                }
            }
        }
        SetExpr::Query(inner) => {
            collect_from_query(inner, rule, source, diags);
        }
        _ => {}
    }
}

/// Check a single branch of a set operation.
/// A branch is flagged when it is a `SetExpr::Query(q)` where `q.limit`
/// or `q.fetch` is set — meaning it was written as a parenthesized
/// sub-select with an explicit LIMIT, e.g. `(SELECT … LIMIT 10)`.
fn check_branch(
    branch: &SetExpr,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    if let SetExpr::Query(q) = branch {
        if q.limit.is_some() || q.fetch.is_some() {
            let (line, col) = find_limit_pos(source);
            diags.push(Diagnostic {
                rule,
                message: "LIMIT inside a UNION/INTERSECT/EXCEPT branch is non-portable — apply LIMIT to the outer query instead".to_string(),
                line,
                col,
            });
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        collect_from_query(subquery, rule, source, diags);
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Find the first occurrence of the LIMIT keyword (case-insensitive,
/// word boundary) in `source`. Returns (1, 1) as fallback.
fn find_limit_pos(source: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let kw = b"LIMIT";
    let kw_len = kw.len();

    let mut i = 0;
    while i + kw_len <= len {
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if before_ok && bytes[i..i + kw_len].eq_ignore_ascii_case(kw) {
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);
            if after_ok {
                return offset_to_line_col(source, i);
            }
        }
        i += 1;
    }

    (1, 1)
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
