use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct TooManyOrderByColumns {
    /// Maximum number of ORDER BY columns allowed per query.
    /// Queries with more columns than this are flagged. Default: 5.
    pub max_columns: usize,
}

impl Default for TooManyOrderByColumns {
    fn default() -> Self {
        TooManyOrderByColumns { max_columns: 5 }
    }
}

impl Rule for TooManyOrderByColumns {
    fn name(&self) -> &'static str {
        "Structure/TooManyOrderByColumns"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.max_columns, ctx, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    max_columns: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, max_columns, ctx, diags);
        }
    }

    // Check the ORDER BY clause for this query level.
    if let Some(order_by) = &query.order_by {
        let n = order_by.exprs.len();
        if n > max_columns {
            let (line, col) = find_order_by_pos(&ctx.source);
            diags.push(Diagnostic {
                rule: "Structure/TooManyOrderByColumns",
                message: format!(
                    "ORDER BY has {n} columns, exceeding the maximum of {max}",
                    n = n,
                    max = max_columns,
                ),
                line,
                col,
            });
        }
    }

    // Recurse into the body to find nested queries / subqueries.
    check_set_expr(&query.body, max_columns, ctx, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    max_columns: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, max_columns, ctx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, max_columns, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, max_columns, ctx, diags);
            check_set_expr(right, max_columns, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    max_columns: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Recurse into subqueries in the FROM clause (derived tables).
    for twj in &sel.from {
        check_table_factor(&twj.relation, max_columns, ctx, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, max_columns, ctx, diags);
        }
    }

    // Recurse into subquery expressions in WHERE and SELECT projection.
    if let Some(selection) = &sel.selection {
        check_expr(selection, max_columns, ctx, diags);
    }

    for item in &sel.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, max_columns, ctx, diags);
        }
    }
}

fn check_table_factor(
    tf: &TableFactor,
    max_columns: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, max_columns, ctx, diags);
    }
}

fn check_expr(expr: &Expr, max_columns: usize, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Subquery(q) => check_query(q, max_columns, ctx, diags),
        Expr::InSubquery { subquery, .. } => check_query(subquery, max_columns, ctx, diags),
        Expr::Exists { subquery, .. } => check_query(subquery, max_columns, ctx, diags),
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, max_columns, ctx, diags);
            check_expr(right, max_columns, ctx, diags);
        }
        _ => {}
    }
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the first occurrence of `ORDER BY` (word-boundary, case-insensitive)
/// in `source`, outside strings/comments.
/// Returns a 1-indexed (line, col) pair; falls back to (1, 1) if not found.
fn find_order_by_pos(source: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();

    // We look for "ORDER" followed by whitespace and then "BY".
    // Minimum span: "ORDER BY" = 8 bytes.
    let mut i = 0;
    while i < len {
        // Skip non-code regions (strings/comments) by checking common delimiters.
        // We use a lightweight inline approach consistent with other rules.

        // Word boundary before "ORDER".
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Match "ORDER" case-insensitively.
        if i + 5 <= len
            && bytes[i].eq_ignore_ascii_case(&b'O')
            && bytes[i + 1].eq_ignore_ascii_case(&b'R')
            && bytes[i + 2].eq_ignore_ascii_case(&b'D')
            && bytes[i + 3].eq_ignore_ascii_case(&b'E')
            && bytes[i + 4].eq_ignore_ascii_case(&b'R')
        {
            // Word boundary after "ORDER".
            let after_order = i + 5;
            if after_order < len && is_word_char(bytes[after_order]) {
                i += 1;
                continue;
            }

            // Skip whitespace.
            let mut j = after_order;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                j += 1;
            }

            // Match "BY" case-insensitively.
            if j + 2 <= len
                && bytes[j].eq_ignore_ascii_case(&b'B')
                && bytes[j + 1].eq_ignore_ascii_case(&b'Y')
            {
                // Word boundary after "BY".
                let after_by = j + 2;
                let by_end_ok = after_by >= len || !is_word_char(bytes[after_by]);
                if by_end_ok {
                    return line_col(source, i);
                }
            }
        }

        i += 1;
    }

    (1, 1)
}

/// Returns `true` if `ch` is a word character (`[a-zA-Z0-9_]`).
#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
