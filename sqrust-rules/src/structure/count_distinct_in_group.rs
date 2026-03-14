use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    DuplicateTreatment, Expr, FunctionArg, FunctionArgExpr, FunctionArguments, GroupByExpr, Query,
    Select, SelectItem, SetExpr, Statement, TableFactor,
};

pub struct CountDistinctInGroup;

impl Rule for CountDistinctInGroup {
    fn name(&self) -> &'static str {
        "Structure/CountDistinctInGroup"
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
    // Collect GROUP BY column names (case-insensitive).
    let group_by_cols = collect_group_by_cols(sel);

    if !group_by_cols.is_empty() {
        // Walk the projection looking for COUNT(DISTINCT <ident>) where <ident>
        // is also in GROUP BY.
        for item in &sel.projection {
            let expr = match item {
                SelectItem::UnnamedExpr(e) => e,
                SelectItem::ExprWithAlias { expr: e, .. } => e,
                _ => continue,
            };

            if let Some(col) = count_distinct_col(expr) {
                let col_upper = col.to_uppercase();
                if group_by_cols.iter().any(|g| g.to_uppercase() == col_upper) {
                    let (line, col_pos) = find_keyword_pos(&ctx.source, "COUNT");
                    diags.push(Diagnostic {
                        rule,
                        message: format!(
                            "COUNT(DISTINCT {col}) with GROUP BY {col} is redundant \
                             — after grouping by {col}, each group has at most one distinct value"
                        ),
                        line,
                        col: col_pos,
                    });
                }
            }
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

// ── helpers ───────────────────────────────────────────────────────────────────

/// Collect all simple identifier names from the GROUP BY clause.
fn collect_group_by_cols(sel: &Select) -> Vec<String> {
    if let GroupByExpr::Expressions(exprs, _) = &sel.group_by {
        exprs
            .iter()
            .filter_map(|e| {
                if let Expr::Identifier(ident) = e {
                    Some(ident.value.clone())
                } else {
                    None
                }
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// If `expr` is `COUNT(DISTINCT <simple_ident>)`, return the identifier name.
/// Returns `None` for any other expression.
fn count_distinct_col(expr: &Expr) -> Option<String> {
    if let Expr::Function(func) = expr {
        let func_name = func
            .name
            .0
            .last()
            .map(|ident| ident.value.to_uppercase())
            .unwrap_or_default();

        if func_name != "COUNT" {
            return None;
        }

        if let FunctionArguments::List(arg_list) = &func.args {
            if arg_list.duplicate_treatment == Some(DuplicateTreatment::Distinct) {
                // Exactly one argument that is a simple identifier.
                if arg_list.args.len() == 1 {
                    if let FunctionArg::Unnamed(FunctionArgExpr::Expr(Expr::Identifier(ident))) =
                        &arg_list.args[0]
                    {
                        return Some(ident.value.clone());
                    }
                }
            }
        }
    }
    None
}

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
