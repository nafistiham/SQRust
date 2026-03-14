use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SetExpr, Statement, TableFactor};

pub struct CorrelatedSubquery;

impl Rule for CorrelatedSubquery {
    fn name(&self) -> &'static str {
        "Structure/CorrelatedSubquery"
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
        SetExpr::Select(sel) => {
            collect_from_select(sel, rule, source, diags);
        }
        SetExpr::Query(inner) => {
            collect_from_query(inner, rule, source, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            collect_from_set_expr(left, rule, source, diags);
            collect_from_set_expr(right, rule, source, diags);
        }
        _ => {}
    }
}

fn collect_from_select(
    sel: &Select,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    // Check WHERE clause for subqueries.
    if let Some(selection) = &sel.selection {
        collect_subqueries_in_expr(selection, rule, source, diags, ClauseKind::Where);
    }

    // Check HAVING clause for subqueries.
    if let Some(having) = &sel.having {
        collect_subqueries_in_expr(having, rule, source, diags, ClauseKind::Having);
    }

    // Recurse into derived tables in FROM to catch nested violations.
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, rule, source, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, rule, source, diags);
        }
    }
}

#[derive(Clone, Copy)]
enum ClauseKind {
    Where,
    Having,
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

/// Walk an expression and emit a diagnostic for each subquery found in
/// WHERE or HAVING clauses (indicating a potential correlated subquery).
fn collect_subqueries_in_expr(
    expr: &Expr,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
    clause: ClauseKind,
) {
    match expr {
        Expr::Subquery(subquery) => {
            let (line, col) = find_subquery_pos(source, clause);
            diags.push(Diagnostic {
                rule,
                message: "Correlated subquery in WHERE/HAVING may execute once per row; consider rewriting as a JOIN or using EXISTS".to_string(),
                line,
                col,
            });
            // Recurse into the subquery itself for nested violations.
            collect_from_query(subquery, rule, source, diags);
        }
        Expr::InSubquery { subquery, expr: inner, .. } => {
            let (line, col) = find_subquery_pos(source, clause);
            diags.push(Diagnostic {
                rule,
                message: "Correlated subquery in WHERE/HAVING may execute once per row; consider rewriting as a JOIN or using EXISTS".to_string(),
                line,
                col,
            });
            collect_subqueries_in_expr(inner, rule, source, diags, clause);
            collect_from_query(subquery, rule, source, diags);
        }
        Expr::Exists { subquery, .. } => {
            let (line, col) = find_subquery_pos(source, clause);
            diags.push(Diagnostic {
                rule,
                message: "Correlated subquery in WHERE/HAVING may execute once per row; consider rewriting as a JOIN or using EXISTS".to_string(),
                line,
                col,
            });
            collect_from_query(subquery, rule, source, diags);
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_subqueries_in_expr(left, rule, source, diags, clause);
            collect_subqueries_in_expr(right, rule, source, diags, clause);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            collect_subqueries_in_expr(inner, rule, source, diags, clause);
        }
        Expr::Nested(inner) => {
            collect_subqueries_in_expr(inner, rule, source, diags, clause);
        }
        _ => {}
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Find the first occurrence of WHERE or HAVING keyword in `source` depending
/// on the clause kind. Returns (1, 1) as fallback.
fn find_subquery_pos(source: &str, clause: ClauseKind) -> (usize, usize) {
    let keyword = match clause {
        ClauseKind::Where => b"WHERE" as &[u8],
        ClauseKind::Having => b"HAVING" as &[u8],
    };
    let bytes = source.as_bytes();
    let len = bytes.len();
    let kw_len = keyword.len();

    let mut i = 0;
    while i + kw_len <= len {
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if before_ok && bytes[i..i + kw_len].eq_ignore_ascii_case(keyword) {
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
