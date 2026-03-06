use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, GroupByExpr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct ExcessiveGroupByColumns {
    /// Maximum number of GROUP BY columns allowed. Queries with more columns
    /// than this are flagged.
    pub max_columns: usize,
}

impl Default for ExcessiveGroupByColumns {
    fn default() -> Self {
        ExcessiveGroupByColumns { max_columns: 5 }
    }
}

impl Rule for ExcessiveGroupByColumns {
    fn name(&self) -> &'static str {
        "ExcessiveGroupByColumns"
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
    // Check the GROUP BY clause.
    if let GroupByExpr::Expressions(exprs, _) = &sel.group_by {
        let n = exprs.len();
        if n > max_columns {
            let (line, col) = find_keyword_pos(&ctx.source, "GROUP");
            diags.push(Diagnostic {
                rule: "ExcessiveGroupByColumns",
                message: format!(
                    "GROUP BY has {n} columns, exceeding the maximum of {max}",
                    n = n,
                    max = max_columns,
                ),
                line,
                col,
            });
        }
    }
    // GroupByExpr::All — treat as 0 columns; no violation.

    // Recurse into subqueries in the FROM clause (derived tables).
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, max_columns, ctx, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, max_columns, ctx, diags);
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

fn recurse_table_factor(
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
