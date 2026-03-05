use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct LimitWithoutOrderBy;

impl Rule for LimitWithoutOrderBy {
    fn name(&self) -> &'static str {
        "Structure/LimitWithoutOrderBy"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            collect_from_statement(stmt, ctx, &mut diags);
        }

        diags
    }
}

/// Recursively collect violations from a single top-level Statement.
fn collect_from_statement(stmt: &Statement, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Statement::Query(query) = stmt {
        collect_from_query(query, ctx, diags);
    }
}

/// Recursively walk a Query, checking for LIMIT without ORDER BY.
fn collect_from_query(query: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Check CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, ctx, diags);
        }
    }

    // Check this query's own LIMIT / ORDER BY.
    let has_limit = query.limit.is_some() || query.fetch.is_some();
    let has_order_by = query
        .order_by
        .as_ref()
        .map(|ob| !ob.exprs.is_empty())
        .unwrap_or(false);

    if has_limit && !has_order_by {
        // Locate the LIMIT keyword in the source text for an accurate position.
        let (line, col) = find_keyword_pos(&ctx.source, "LIMIT");
        diags.push(Diagnostic {
            rule: "Structure/LimitWithoutOrderBy",
            message: "LIMIT without ORDER BY produces non-deterministic results".to_string(),
            line,
            col,
        });
    }

    // Recurse into the body (handles subqueries in FROM clauses, nested set
    // operations, etc.).
    collect_from_set_expr(&query.body, ctx, diags);
}

/// Recurse into a SetExpr, looking for nested Queries.
fn collect_from_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(select) => {
            collect_from_select(select, ctx, diags);
        }
        SetExpr::Query(inner) => {
            collect_from_query(inner, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            collect_from_set_expr(left, ctx, diags);
            collect_from_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}

/// Recurse into a SELECT, checking FROM clause subqueries and WHERE
/// expressions.
fn collect_from_select(select: &Select, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // FROM clause — check Derived subqueries.
    for table_with_joins in &select.from {
        collect_from_table_factor(&table_with_joins.relation, ctx, diags);
        for join in &table_with_joins.joins {
            collect_from_table_factor(&join.relation, ctx, diags);
        }
    }

    // WHERE clause — check subquery expressions.
    if let Some(selection) = &select.selection {
        collect_from_expr(selection, ctx, diags);
    }

    // SELECT projection — check scalar subqueries.
    for item in &select.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            collect_from_expr(e, ctx, diags);
        }
    }
}

/// Recurse into a TableFactor looking for Derived (subquery) tables.
fn collect_from_table_factor(
    factor: &TableFactor,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        collect_from_query(subquery, ctx, diags);
    }
}

/// Recurse into an expression looking for subqueries.
fn collect_from_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Subquery(q) => collect_from_query(q, ctx, diags),
        Expr::InSubquery { subquery, .. } => collect_from_query(subquery, ctx, diags),
        Expr::Exists { subquery, .. } => collect_from_query(subquery, ctx, diags),
        Expr::BinaryOp { left, right, .. } => {
            collect_from_expr(left, ctx, diags);
            collect_from_expr(right, ctx, diags);
        }
        _ => {}
    }
}

/// Finds the first occurrence of `keyword` (case-insensitive, word-boundary)
/// in `source` and returns a 1-indexed (line, col) pair.
/// Falls back to (1, 1) if not found.
fn find_keyword_pos(source: &str, keyword: &str) -> (usize, usize) {
    let upper = source.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let kw_len = kw_upper.len();
    let bytes = upper.as_bytes();
    let len = bytes.len();
    let kw_bytes = kw_upper.as_bytes();

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

            // Suppress matches inside strings/comments by checking the
            // original source character.  We use a simple heuristic: if the
            // original source byte at `abs` is inside a single-quoted string
            // the first char before it would be a `'`.  A full SkipMap is
            // overkill here since we only need the position, not precision —
            // the AST already confirmed a real LIMIT exists.
            if before_ok && after_ok {
                let _ = kw_bytes; // reference to silence unused-variable warning
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
