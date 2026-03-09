use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct NestedCaseInElse;

impl Rule for NestedCaseInElse {
    fn name(&self) -> &'static str {
        "Structure/NestedCaseInElse"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        // Counter tracks how many ELSE keywords we've already used so we can
        // pinpoint the correct ELSE that introduces the nested CASE.
        let mut else_count = 0usize;
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, ctx, &mut else_count, &mut diags);
            }
        }
        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    q: &Query,
    ctx: &FileContext,
    else_count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &q.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, else_count, diags);
        }
    }
    check_set_expr(&q.body, ctx, else_count, diags);

    // ORDER BY expressions.
    if let Some(order_by) = &q.order_by {
        for ob_expr in &order_by.exprs {
            walk_expr(&ob_expr.expr, ctx, else_count, diags);
        }
    }
}

fn check_set_expr(
    expr: &SetExpr,
    ctx: &FileContext,
    else_count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, ctx, else_count, diags),
        SetExpr::Query(inner) => check_query(inner, ctx, else_count, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, else_count, diags);
            check_set_expr(right, ctx, else_count, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    ctx: &FileContext,
    else_count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // Projection.
    for item in &sel.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            walk_expr(e, ctx, else_count, diags);
        }
    }

    // WHERE.
    if let Some(selection) = &sel.selection {
        walk_expr(selection, ctx, else_count, diags);
    }

    // HAVING.
    if let Some(having) = &sel.having {
        walk_expr(having, ctx, else_count, diags);
    }

    // GROUP BY.
    if let sqlparser::ast::GroupByExpr::Expressions(exprs, _) = &sel.group_by {
        for e in exprs {
            walk_expr(e, ctx, else_count, diags);
        }
    }

    // FROM — recurse into subqueries.
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, ctx, else_count, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, ctx, else_count, diags);
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    ctx: &FileContext,
    else_count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, ctx, else_count, diags);
    }
}

/// Walk an expression. When we encounter a CASE whose ELSE is itself a CASE,
/// emit a diagnostic, then continue recursing into both branches.
fn walk_expr(
    expr: &Expr,
    ctx: &FileContext,
    else_count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            // Recurse into operand, WHEN/THEN branches first (they appear
            // before ELSE in source text, so their ELSE keywords are counted
            // before the outer ELSE).
            if let Some(op) = operand {
                walk_expr(op, ctx, else_count, diags);
            }
            for cond in conditions {
                walk_expr(cond, ctx, else_count, diags);
            }
            for res in results {
                walk_expr(res, ctx, else_count, diags);
            }

            if let Some(else_expr) = else_result {
                // Check if the ELSE value is itself a CASE.
                if matches!(else_expr.as_ref(), Expr::Case { .. }) {
                    // Find the Nth ELSE keyword (the one that introduces this
                    // nested CASE) and emit a diagnostic at it.
                    let nth = *else_count;
                    let offset =
                        find_nth_keyword(&ctx.source, "ELSE", nth).unwrap_or(0);
                    let (line, col) = offset_to_line_col(&ctx.source, offset);
                    diags.push(Diagnostic {
                        rule: "Structure/NestedCaseInElse",
                        message:
                            "CASE expression has a nested CASE in its ELSE clause; \
                             flatten with additional WHEN branches instead"
                                .to_string(),
                        line,
                        col,
                    });
                }
                // Count the ELSE keyword we just processed.
                *else_count += 1;
                // Recurse into the ELSE expression (catches deeper nesting).
                walk_expr(else_expr, ctx, else_count, diags);
            }
        }

        // Pass-through recursion for other expression types.
        Expr::BinaryOp { left, right, .. } => {
            walk_expr(left, ctx, else_count, diags);
            walk_expr(right, ctx, else_count, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => walk_expr(inner, ctx, else_count, diags),
        Expr::Nested(inner) => walk_expr(inner, ctx, else_count, diags),
        Expr::Cast { expr: inner, .. } => walk_expr(inner, ctx, else_count, diags),
        Expr::IsNull(inner) | Expr::IsNotNull(inner) => walk_expr(inner, ctx, else_count, diags),
        Expr::Between {
            expr: e,
            low,
            high,
            ..
        } => {
            walk_expr(e, ctx, else_count, diags);
            walk_expr(low, ctx, else_count, diags);
            walk_expr(high, ctx, else_count, diags);
        }
        Expr::InList { expr: inner, list, .. } => {
            walk_expr(inner, ctx, else_count, diags);
            for e in list {
                walk_expr(e, ctx, else_count, diags);
            }
        }
        Expr::Function(f) => {
            if let sqlparser::ast::FunctionArguments::List(arg_list) = &f.args {
                for arg in &arg_list.args {
                    if let sqlparser::ast::FunctionArg::Unnamed(
                        sqlparser::ast::FunctionArgExpr::Expr(e),
                    ) = arg
                    {
                        walk_expr(e, ctx, else_count, diags);
                    }
                }
            }
        }
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            check_query(q, ctx, else_count, diags);
        }
        _ => {}
    }
}

// ── Source-text helpers ───────────────────────────────────────────────────────

/// Find the `nth` (0-indexed) whole-word, case-insensitive occurrence of
/// `keyword` in `source`, skipping inside strings/comments.
/// Returns `Some(byte_offset)` or `None`.
fn find_nth_keyword(source: &str, keyword: &str, nth: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let kw: Vec<u8> = keyword.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let kw_len = kw.len();
    let src_len = bytes.len();
    let skip = SkipMap::build(source);

    let mut count = 0usize;
    let mut i = 0usize;

    while i + kw_len <= src_len {
        if !skip.is_code(i) {
            i += 1;
            continue;
        }

        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(kw.iter())
            .all(|(&a, &b)| a.to_ascii_uppercase() == b);

        if matches {
            let end = i + kw_len;
            let after_ok = end >= src_len || !is_word_char(bytes[end]);
            let all_code = (i..end).all(|k| skip.is_code(k));

            if after_ok && all_code {
                if count == nth {
                    return Some(i);
                }
                count += 1;
            }
        }

        i += 1;
    }

    None
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let safe = offset.min(source.len());
    let before = &source[..safe];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| safe - p - 1).unwrap_or(safe) + 1;
    (line, col)
}
