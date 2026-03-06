use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct LargeInList {
    /// Maximum number of values allowed in an IN list.
    /// IN lists with more values than this are flagged.
    pub max_values: usize,
}

impl Default for LargeInList {
    fn default() -> Self {
        LargeInList { max_values: 10 }
    }
}

impl Rule for LargeInList {
    fn name(&self) -> &'static str {
        "LargeInList"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // Track how many IN keywords we've consumed so we can map each
        // violation to the correct source position.
        let mut in_occurrence: usize = 0;

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.max_values, ctx, &mut in_occurrence, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    max: usize,
    ctx: &FileContext,
    occurrence: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, max, ctx, occurrence, diags);
        }
    }

    check_set_expr(&query.body, max, ctx, occurrence, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    max: usize,
    ctx: &FileContext,
    occurrence: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, max, ctx, occurrence, diags),
        SetExpr::Query(inner) => check_query(inner, max, ctx, occurrence, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, max, ctx, occurrence, diags);
            check_set_expr(right, max, ctx, occurrence, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    max: usize,
    ctx: &FileContext,
    occurrence: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // SELECT projection.
    for item in &sel.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, max, ctx, occurrence, diags);
        }
    }

    // WHERE clause.
    if let Some(selection) = &sel.selection {
        check_expr(selection, max, ctx, occurrence, diags);
    }

    // FROM / JOIN — recurse into derived tables.
    for twj in &sel.from {
        check_table_factor(&twj.relation, max, ctx, occurrence, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, max, ctx, occurrence, diags);
        }
    }
}

fn check_table_factor(
    tf: &TableFactor,
    max: usize,
    ctx: &FileContext,
    occurrence: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, max, ctx, occurrence, diags);
    }
}

fn check_expr(
    expr: &Expr,
    max: usize,
    ctx: &FileContext,
    occurrence: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::InList { expr: inner, list, .. } => {
            // Consume one IN occurrence.
            let occ = *occurrence;
            *occurrence += 1;

            let n = list.len();
            if n > max {
                let (line, col) = find_nth_keyword_pos(&ctx.source, "IN", occ);
                diags.push(Diagnostic {
                    rule: "LargeInList",
                    message: format!(
                        "IN list has {n} values, exceeding the maximum of {max}"
                    ),
                    line,
                    col,
                });
            }

            // Recurse into the expression being tested and the list values.
            check_expr(inner, max, ctx, occurrence, diags);
            for val in list {
                check_expr(val, max, ctx, occurrence, diags);
            }
        }

        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, max, ctx, occurrence, diags);
            check_expr(right, max, ctx, occurrence, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, max, ctx, occurrence, diags);
        }
        Expr::Subquery(q) => check_query(q, max, ctx, occurrence, diags),
        Expr::InSubquery { subquery, expr: e, .. } => {
            check_expr(e, max, ctx, occurrence, diags);
            check_query(subquery, max, ctx, occurrence, diags);
        }
        Expr::Exists { subquery, .. } => check_query(subquery, max, ctx, occurrence, diags),
        Expr::Nested(inner) => check_expr(inner, max, ctx, occurrence, diags),
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                check_expr(op, max, ctx, occurrence, diags);
            }
            for cond in conditions {
                check_expr(cond, max, ctx, occurrence, diags);
            }
            for res in results {
                check_expr(res, max, ctx, occurrence, diags);
            }
            if let Some(els) = else_result {
                check_expr(els, max, ctx, occurrence, diags);
            }
        }
        Expr::Function(f) => {
            use sqlparser::ast::{FunctionArg, FunctionArgExpr, FunctionArguments};
            if let FunctionArguments::List(list) = &f.args {
                for arg in &list.args {
                    let arg_expr = match arg {
                        FunctionArg::Unnamed(e) => Some(e),
                        FunctionArg::Named { arg: e, .. } => Some(e),
                        FunctionArg::ExprNamed { arg: e, .. } => Some(e),
                    };
                    if let Some(FunctionArgExpr::Expr(inner)) = arg_expr {
                        check_expr(inner, max, ctx, occurrence, diags);
                    }
                }
            }
        }
        _ => {}
    }
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the `nth` occurrence (0-indexed) of a keyword (case-insensitive,
/// word-boundary, outside strings/comments) in `source`.
/// Returns a 1-indexed (line, col) pair. Falls back to (1, 1) if not found.
fn find_nth_keyword_pos(source: &str, keyword: &str, nth: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);
    let kw_upper: Vec<u8> = keyword.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let kw_len = kw_upper.len();

    let mut count = 0usize;
    let mut i = 0;
    while i + kw_len <= len {
        if !skip_map.is_code(i) {
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
            .zip(kw_upper.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if matches {
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);
            let all_code = (i..i + kw_len).all(|k| skip_map.is_code(k));

            if after_ok && all_code {
                if count == nth {
                    return line_col(source, i);
                }
                count += 1;
            }
        }

        i += 1;
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
