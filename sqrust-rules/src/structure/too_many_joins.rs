use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct TooManyJoins {
    /// Maximum number of JOINs allowed per query. Queries with more JOINs than
    /// this are flagged.
    pub max_joins: usize,
}

impl Default for TooManyJoins {
    fn default() -> Self {
        TooManyJoins { max_joins: 5 }
    }
}

impl Rule for TooManyJoins {
    fn name(&self) -> &'static str {
        "TooManyJoins"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.max_joins, ctx, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    max_joins: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, max_joins, ctx, diags);
        }
    }

    check_set_expr(&query.body, max_joins, ctx, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    max_joins: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, max_joins, ctx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, max_joins, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, max_joins, ctx, diags);
            check_set_expr(right, max_joins, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    max_joins: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Count total JOINs across all FROM items for this SELECT.
    let n: usize = sel.from.iter().map(|twj| twj.joins.len()).sum();

    if n > max_joins {
        let (line, col) = find_keyword_pos(&ctx.source, "JOIN");
        diags.push(Diagnostic {
            rule: "TooManyJoins",
            message: format!(
                "Query has {n} JOINs, exceeding the maximum of {max}",
                n = n,
                max = max_joins,
            ),
            line,
            col,
        });
    }

    // Recurse into subqueries in the FROM clause (derived tables).
    for twj in &sel.from {
        check_table_factor(&twj.relation, max_joins, ctx, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, max_joins, ctx, diags);
        }
    }

    // Recurse into subquery expressions in WHERE and SELECT projection.
    if let Some(selection) = &sel.selection {
        check_expr(selection, max_joins, ctx, diags);
    }

    for item in &sel.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, max_joins, ctx, diags);
        }
    }
}

fn check_table_factor(
    tf: &TableFactor,
    max_joins: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, max_joins, ctx, diags);
    }
}

fn check_expr(expr: &Expr, max_joins: usize, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Subquery(q) => check_query(q, max_joins, ctx, diags),
        Expr::InSubquery { subquery, .. } => check_query(subquery, max_joins, ctx, diags),
        Expr::Exists { subquery, .. } => check_query(subquery, max_joins, ctx, diags),
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, max_joins, ctx, diags);
            check_expr(right, max_joins, ctx, diags);
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
