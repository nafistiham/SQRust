use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct HavingWithoutSelectAgg;

impl Rule for HavingWithoutSelectAgg {
    fn name(&self) -> &'static str {
        "Structure/HavingWithoutSelectAgg"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.name(), ctx, &mut diags);
            }
        }
        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, ctx, diags);
        }
    }
    check_set_expr(&query.body, rule, ctx, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, rule, ctx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, rule, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, rule, ctx, diags);
            check_set_expr(right, rule, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(having) = &sel.having {
        // Only flag if HAVING contains aggregate(s) but SELECT has none.
        if has_aggregate(having) && !select_has_aggregate(sel) {
            let (line, col) = find_keyword_pos(&ctx.source, "HAVING");
            diags.push(Diagnostic {
                rule,
                message: "HAVING uses aggregate function(s) but SELECT list has no aggregates \
                          — consider moving the filter to WHERE or adding the aggregate to SELECT"
                    .to_string(),
                line,
                col,
            });
        }
    }

    // Recurse into subqueries in FROM.
    for table_with_joins in &sel.from {
        recurse_table_factor(&table_with_joins.relation, rule, ctx, diags);
        for join in &table_with_joins.joins {
            recurse_table_factor(&join.relation, rule, ctx, diags);
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, rule, ctx, diags);
    }
}

// ── aggregate detection ───────────────────────────────────────────────────────

/// Returns `true` if `expr` contains a call to a known aggregate function.
fn has_aggregate(expr: &Expr) -> bool {
    match expr {
        Expr::Function(func) => {
            let name = func
                .name
                .0
                .last()
                .map(|i| i.value.to_uppercase())
                .unwrap_or_default();
            matches!(
                name.as_str(),
                "COUNT" | "SUM" | "AVG" | "MIN" | "MAX" | "STDDEV" | "VARIANCE"
            )
        }
        Expr::BinaryOp { left, right, .. } => has_aggregate(left) || has_aggregate(right),
        Expr::UnaryOp { expr, .. } => has_aggregate(expr),
        Expr::Nested(e) => has_aggregate(e),
        Expr::Between {
            expr, low, high, ..
        } => has_aggregate(expr) || has_aggregate(low) || has_aggregate(high),
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            operand.as_ref().map_or(false, |e| has_aggregate(e))
                || conditions.iter().any(|e| has_aggregate(e))
                || results.iter().any(|e| has_aggregate(e))
                || else_result.as_ref().map_or(false, |e| has_aggregate(e))
        }
        _ => false,
    }
}

/// Returns `true` if any item in the SELECT projection contains an aggregate call.
fn select_has_aggregate(sel: &Select) -> bool {
    for item in &sel.projection {
        let expr = match item {
            SelectItem::UnnamedExpr(e) => e,
            SelectItem::ExprWithAlias { expr: e, .. } => e,
            _ => continue,
        };
        if has_aggregate(expr) {
            return true;
        }
    }
    false
}

// ── position helpers ──────────────────────────────────────────────────────────

/// Find the first occurrence of a keyword (case-insensitive, word-boundary) in
/// `source`. Returns a 1-indexed (line, col) pair. Falls back to (1, 1).
fn find_keyword_pos(source: &str, keyword: &str) -> (usize, usize) {
    let upper = source.to_uppercase();
    let kw_upper = keyword.to_uppercase();
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

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
