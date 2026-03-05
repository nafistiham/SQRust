use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct SubqueryInSelect;

impl Rule for SubqueryInSelect {
    fn name(&self) -> &'static str {
        "Structure/SubqueryInSelect"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.name(), &ctx.source, &mut diags);
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
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs first.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, source, diags);
        }
    }
    check_set_expr(&query.body, rule, source, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, rule, source, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, rule, source, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, rule, source, diags);
            check_set_expr(right, rule, source, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    // Check each item in the projection list for a scalar subquery.
    for item in &sel.projection {
        let expr = match item {
            SelectItem::UnnamedExpr(e) => Some(e),
            SelectItem::ExprWithAlias { expr, .. } => Some(expr),
            _ => None,
        };

        if let Some(Expr::Subquery(subquery)) = expr {
            let (line, col) = find_subquery_pos(source, subquery);
            diags.push(Diagnostic {
                rule,
                message: "Scalar subquery in SELECT list may cause N+1 query performance issues; consider using a JOIN".to_string(),
                line,
                col,
            });
            // Recurse into the subquery's own SELECT list (nested pattern).
            check_query(subquery, rule, source, diags);
        }
    }

    // Recurse into subqueries in the FROM clause.
    for table in &sel.from {
        recurse_table_factor(&table.relation, rule, source, diags);
        for join in &table.joins {
            recurse_table_factor(&join.relation, rule, source, diags);
        }
    }

    // Recurse into the WHERE clause expressions to catch any scalar subqueries
    // that appear in nested queries reached through WHERE (not flagged, just
    // walked so nested SELECT-list subqueries can be found).
    if let Some(selection) = &sel.selection {
        recurse_expr_for_queries(selection, rule, source, diags);
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, rule, source, diags);
    }
}

/// Walk an expression only to find nested Query nodes (e.g. in WHERE/IN/EXISTS)
/// so that any SELECT-list subqueries inside those are checked. We do NOT flag
/// the expression itself here — only SELECT-list items are flagged.
fn recurse_expr_for_queries(
    expr: &Expr,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Subquery(q) => check_query(q, rule, source, diags),
        Expr::InSubquery { subquery, .. } => check_query(subquery, rule, source, diags),
        Expr::Exists { subquery, .. } => check_query(subquery, rule, source, diags),
        Expr::BinaryOp { left, right, .. } => {
            recurse_expr_for_queries(left, rule, source, diags);
            recurse_expr_for_queries(right, rule, source, diags);
        }
        _ => {}
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Find the position of the opening `(SELECT` for a scalar subquery.
/// Scans the source for `(` followed by optional whitespace followed by SELECT.
/// Falls back to (1, 1) if not found.
fn find_subquery_pos(source: &str, _query: &Query) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();

    let mut i = 0;
    while i < len {
        if bytes[i] == b'(' {
            // Scan forward past optional whitespace.
            let mut j = i + 1;
            while j < len
                && (bytes[j] == b' '
                    || bytes[j] == b'\t'
                    || bytes[j] == b'\n'
                    || bytes[j] == b'\r')
            {
                j += 1;
            }

            // Check for SELECT keyword (case-insensitive, word-boundary after).
            let kw = b"SELECT";
            let kw_len = kw.len();
            if j + kw_len <= len {
                let matches = bytes[j..j + kw_len]
                    .iter()
                    .zip(kw.iter())
                    .all(|(a, b)| a.eq_ignore_ascii_case(b));

                let boundary_after = j + kw_len >= len || {
                    let nb = bytes[j + kw_len];
                    !nb.is_ascii_alphanumeric() && nb != b'_'
                };

                if matches && boundary_after {
                    return line_col(source, i);
                }
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
