use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Distinct, Expr, OrderBy, Query, Select, SelectItem, SetExpr, Statement,
    TableFactor};
use std::collections::HashSet;

pub struct SelectDistinctOrderBy;

impl Rule for SelectDistinctOrderBy {
    fn name(&self) -> &'static str {
        "Ambiguous/SelectDistinctOrderBy"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, ctx, &mut diags);
            }
        }
        diags
    }
}

fn check_query(query: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, diags);
        }
    }
    check_set_expr(&query.body, query.order_by.as_ref(), ctx, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    order_by: Option<&OrderBy>,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(select) => {
            check_select(select, order_by, ctx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, None, ctx, diags);
            check_set_expr(right, None, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(
    select: &Select,
    order_by: Option<&OrderBy>,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Recurse into FROM subqueries.
    for twj in &select.from {
        recurse_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, ctx, diags);
        }
    }

    // Only flag when DISTINCT is present.
    if !matches!(select.distinct, Some(Distinct::Distinct)) {
        return;
    }

    let Some(ob) = order_by else {
        return;
    };

    if ob.exprs.is_empty() {
        return;
    }

    // Collect SELECT column names (lowercased).
    let select_names = collect_select_names(&select.projection);

    // If wildcard is in projection, any ORDER BY column is fine.
    if select_names.contains("*") {
        return;
    }

    // For each ORDER BY expression that is a plain identifier, check if it
    // appears in the SELECT list.
    for order_expr in &ob.exprs {
        if let Some(col_name) = extract_identifier_name(&order_expr.expr) {
            if !select_names.contains(col_name.as_str()) {
                let (line, col) = find_nth_keyword_position(&ctx.source, "ORDER BY", 0);
                diags.push(Diagnostic {
                    rule: "Ambiguous/SelectDistinctOrderBy",
                    message: format!(
                        "SELECT DISTINCT with ORDER BY column '{}' not in SELECT list \
                         — behavior is undefined or an error in most databases",
                        col_name
                    ),
                    line,
                    col,
                });
            }
        }
    }
}

fn recurse_table_factor(factor: &TableFactor, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = factor {
        check_query(subquery, ctx, diags);
    }
}

/// Returns a set of lowercase column/alias names from the SELECT projection.
/// Includes "*" if a wildcard is present.
fn collect_select_names(projection: &[SelectItem]) -> HashSet<String> {
    let mut names = HashSet::new();
    for item in projection {
        match item {
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => {
                names.insert("*".to_string());
            }
            SelectItem::UnnamedExpr(expr) => {
                if let Some(name) = extract_identifier_name(expr) {
                    names.insert(name);
                }
            }
            SelectItem::ExprWithAlias { alias, .. } => {
                names.insert(alias.value.to_lowercase());
            }
        }
    }
    names
}

/// Extracts the lowercase identifier name from a simple column reference.
/// Returns `None` for complex expressions like function calls or arithmetic.
fn extract_identifier_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Identifier(ident) => Some(ident.value.to_lowercase()),
        Expr::CompoundIdentifier(parts) => {
            // Use the last part (the column name).
            parts.last().map(|i| i.value.to_lowercase())
        }
        _ => None,
    }
}

/// Finds the `n`-th (0-indexed) case-insensitive occurrence of `keyword`
/// in `source` at a word boundary and returns a 1-indexed (line, col).
/// Falls back to (1, 1) if not found.
fn find_nth_keyword_position(source: &str, keyword: &str, n: usize) -> (usize, usize) {
    let upper = source.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let bytes = upper.as_bytes();
    let kw_bytes = kw_upper.as_bytes();
    let kw_len = kw_bytes.len();

    let mut found = 0usize;
    let mut i = 0;
    while i + kw_len <= bytes.len() {
        if bytes[i..i + kw_len] == *kw_bytes {
            let before_ok =
                i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let after = i + kw_len;
            let after_ok = after >= bytes.len()
                || (!bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_');
            if before_ok && after_ok {
                if found == n {
                    return offset_to_line_col(source, i);
                }
                found += 1;
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
