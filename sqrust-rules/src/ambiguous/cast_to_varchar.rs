use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{DataType, Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct CastToVarchar;

impl Rule for CastToVarchar {
    fn name(&self) -> &'static str {
        "Ambiguous/CastToVarchar"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let mut cast_counter = 0usize;
        for stmt in &ctx.statements {
            collect_from_statement(stmt, ctx, &mut cast_counter, &mut diags);
        }
        diags
    }
}

fn collect_from_statement(
    stmt: &Statement,
    ctx: &FileContext,
    cast_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let Statement::Query(query) = stmt {
        collect_from_query(query, ctx, cast_counter, diags);
    }
}

fn collect_from_query(
    query: &Query,
    ctx: &FileContext,
    cast_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, ctx, cast_counter, diags);
        }
    }
    collect_from_set_expr(&query.body, ctx, cast_counter, diags);
}

fn collect_from_set_expr(
    expr: &SetExpr,
    ctx: &FileContext,
    cast_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(select) => {
            collect_from_select(select, ctx, cast_counter, diags);
        }
        SetExpr::Query(inner) => {
            collect_from_query(inner, ctx, cast_counter, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            collect_from_set_expr(left, ctx, cast_counter, diags);
            collect_from_set_expr(right, ctx, cast_counter, diags);
        }
        _ => {}
    }
}

fn collect_from_select(
    select: &Select,
    ctx: &FileContext,
    cast_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // Check projection.
    for item in &select.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, ctx, cast_counter, diags);
        }
    }

    // Check FROM subqueries.
    for twj in &select.from {
        collect_from_table_factor(&twj.relation, ctx, cast_counter, diags);
        for join in &twj.joins {
            collect_from_table_factor(&join.relation, ctx, cast_counter, diags);
        }
    }

    // Check WHERE.
    if let Some(selection) = &select.selection {
        check_expr(selection, ctx, cast_counter, diags);
    }

    // Check HAVING.
    if let Some(having) = &select.having {
        check_expr(having, ctx, cast_counter, diags);
    }
}

fn collect_from_table_factor(
    factor: &TableFactor,
    ctx: &FileContext,
    cast_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        collect_from_query(subquery, ctx, cast_counter, diags);
    }
}

fn check_expr(
    expr: &Expr,
    ctx: &FileContext,
    cast_counter: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Cast { data_type, expr: inner, .. } => {
            // Recurse into the cast expression first.
            check_expr(inner, ctx, cast_counter, diags);

            if is_varchar_without_length(data_type) {
                let (line, col) = find_nth_occurrence(&ctx.source, "CAST", *cast_counter);
                *cast_counter += 1;
                diags.push(Diagnostic {
                    rule: "Ambiguous/CastToVarchar",
                    message: "CAST to VARCHAR without length — default length varies by dialect \
                               (1, 255, or unlimited); specify explicit length e.g. VARCHAR(255)"
                        .to_string(),
                    line,
                    col,
                });
            } else {
                // Still count as a CAST occurrence so subsequent ones get correct positions.
                *cast_counter += 1;
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, ctx, cast_counter, diags);
            check_expr(right, ctx, cast_counter, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, ctx, cast_counter, diags);
        }
        Expr::Nested(inner) => {
            check_expr(inner, ctx, cast_counter, diags);
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                check_expr(op, ctx, cast_counter, diags);
            }
            for cond in conditions {
                check_expr(cond, ctx, cast_counter, diags);
            }
            for result in results {
                check_expr(result, ctx, cast_counter, diags);
            }
            if let Some(else_e) = else_result {
                check_expr(else_e, ctx, cast_counter, diags);
            }
        }
        Expr::InList { expr: inner, list, .. } => {
            check_expr(inner, ctx, cast_counter, diags);
            for e in list {
                check_expr(e, ctx, cast_counter, diags);
            }
        }
        Expr::Between { expr: inner, low, high, .. } => {
            check_expr(inner, ctx, cast_counter, diags);
            check_expr(low, ctx, cast_counter, diags);
            check_expr(high, ctx, cast_counter, diags);
        }
        Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
            check_expr(inner, ctx, cast_counter, diags);
        }
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, ctx, cast_counter, diags);
        }
        _ => {}
    }
}

/// Returns `true` when `data_type` is a character-varying type without a
/// length specifier — i.e., one that has an implementation-defined default.
fn is_varchar_without_length(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Varchar(None)
            | DataType::Nvarchar(None)
            | DataType::CharVarying(None)
            | DataType::Char(None)
    )
}

/// Finds the `n`-th (0-indexed) case-insensitive occurrence of `keyword`
/// at a word boundary in `source` and returns a 1-indexed (line, col).
/// Falls back to (1, 1) if not found.
fn find_nth_occurrence(source: &str, keyword: &str, n: usize) -> (usize, usize) {
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
