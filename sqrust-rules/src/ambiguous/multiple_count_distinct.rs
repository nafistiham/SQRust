use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    DuplicateTreatment, Expr, FunctionArguments, Query, Select, SelectItem, SetExpr, Statement,
    TableFactor,
};

pub struct MultipleCountDistinct;

impl Rule for MultipleCountDistinct {
    fn name(&self) -> &'static str {
        "Ambiguous/MultipleCountDistinct"
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

fn collect_from_statement(stmt: &Statement, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Statement::Query(query) = stmt {
        collect_from_query(query, ctx, diags);
    }
}

fn collect_from_query(query: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, ctx, diags);
        }
    }
    collect_from_set_expr(&query.body, ctx, diags);
}

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

fn collect_from_select(select: &Select, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Count COUNT(DISTINCT ...) expressions in this SELECT's projection.
    let mut count_distinct_n = 0usize;

    for item in &select.projection {
        let expr = match item {
            SelectItem::UnnamedExpr(e) => e,
            SelectItem::ExprWithAlias { expr: e, .. } => e,
            _ => continue,
        };
        if is_count_distinct(expr) {
            count_distinct_n += 1;
        }
    }

    if count_distinct_n > 1 {
        let (line, col) = find_keyword_pos(&ctx.source, "SELECT");
        diags.push(Diagnostic {
            rule: "Ambiguous/MultipleCountDistinct",
            message: "Multiple COUNT(DISTINCT ...) in a single SELECT may produce approximate or incorrect results in some databases — consider restructuring with subqueries".to_string(),
            line,
            col,
        });
    }

    // Recurse into FROM subqueries.
    for twj in &select.from {
        collect_from_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            collect_from_table_factor(&join.relation, ctx, diags);
        }
    }

    // Recurse into WHERE subqueries.
    if let Some(selection) = &select.selection {
        collect_subqueries_from_expr(selection, ctx, diags);
    }

    // Recurse into HAVING subqueries.
    if let Some(having) = &select.having {
        collect_subqueries_from_expr(having, ctx, diags);
    }
}

fn collect_from_table_factor(
    factor: &TableFactor,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        collect_from_query(subquery, ctx, diags);
    }
}

/// Returns true if `expr` is `COUNT(DISTINCT ...)`.
fn is_count_distinct(expr: &Expr) -> bool {
    if let Expr::Function(func) = expr {
        let func_name = func
            .name
            .0
            .last()
            .map(|ident| ident.value.to_uppercase())
            .unwrap_or_default();

        if func_name != "COUNT" {
            return false;
        }

        if let FunctionArguments::List(arg_list) = &func.args {
            return arg_list.duplicate_treatment == Some(DuplicateTreatment::Distinct);
        }
    }
    false
}

/// Recurse into subqueries nested inside expressions (WHERE / HAVING clauses).
fn collect_subqueries_from_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, ctx, diags);
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_subqueries_from_expr(left, ctx, diags);
            collect_subqueries_from_expr(right, ctx, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            collect_subqueries_from_expr(inner, ctx, diags);
        }
        Expr::Nested(inner) => {
            collect_subqueries_from_expr(inner, ctx, diags);
        }
        _ => {}
    }
}

/// Finds the first occurrence of a keyword (case-insensitive, word-boundary)
/// in `source` and returns a 1-indexed (line, col). Falls back to (1, 1).
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

/// Converts a byte offset to 1-indexed (line, col).
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
