use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct InSingleValue;

impl Rule for InSingleValue {
    fn name(&self) -> &'static str {
        "InSingleValue"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        // Collect all byte offsets of `IN` keywords (word-boundary, outside
        // strings/comments, case-insensitive) in source order.
        let in_offsets = collect_in_offsets(&ctx.source);
        let mut in_index: usize = 0;
        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(
                    query,
                    self.name(),
                    &ctx.source,
                    &in_offsets,
                    &mut in_index,
                    &mut diags,
                );
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, source, offsets, idx, diags);
        }
    }
    check_set_expr(&query.body, rule, source, offsets, idx, diags);
}

fn check_set_expr(
    body: &SetExpr,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match body {
        SetExpr::Select(sel) => check_select(sel, rule, source, offsets, idx, diags),
        SetExpr::Query(q) => check_query(q, rule, source, offsets, idx, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, rule, source, offsets, idx, diags);
            check_set_expr(right, rule, source, offsets, idx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // Recurse into subqueries in the FROM clause.
    for table in &sel.from {
        recurse_table_factor(&table.relation, rule, source, offsets, idx, diags);
        for join in &table.joins {
            recurse_table_factor(&join.relation, rule, source, offsets, idx, diags);
        }
    }

    // Check the WHERE clause.
    if let Some(selection) = &sel.selection {
        check_expr(selection, rule, source, offsets, idx, diags);
    }

    // Check expressions in the projection (HAVING-style subqueries, etc.).
    for item in &sel.projection {
        use sqlparser::ast::SelectItem;
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                check_expr(e, rule, source, offsets, idx, diags);
            }
            _ => {}
        }
    }

    // Check HAVING clause.
    if let Some(having) = &sel.having {
        check_expr(having, rule, source, offsets, idx, diags);
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, rule, source, offsets, idx, diags);
    }
}

fn check_expr(
    expr: &Expr,
    rule: &'static str,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::InList {
            list,
            negated,
            expr: inner,
        } => {
            // Check the inner expression first (it may contain nested IN).
            check_expr(inner, rule, source, offsets, idx, diags);

            if !negated && list.len() == 1 {
                // Consume the next IN offset as a violation.
                let offset = offsets.get(*idx).copied().unwrap_or(0);
                let (line, col) = line_col(source, offset);
                diags.push(Diagnostic {
                    rule,
                    message: "IN list with a single value; use = instead".to_string(),
                    line,
                    col,
                });
                *idx += 1;
            } else {
                // Consume the IN offset without flagging.
                if *idx < offsets.len() {
                    *idx += 1;
                }
            }

            // Recurse into the list elements.
            for e in list {
                check_expr(e, rule, source, offsets, idx, diags);
            }
        }

        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, rule, source, offsets, idx, diags);
            check_expr(right, rule, source, offsets, idx, diags);
        }

        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, rule, source, offsets, idx, diags);
        }

        Expr::Nested(inner) => {
            check_expr(inner, rule, source, offsets, idx, diags);
        }

        Expr::Subquery(q) => {
            check_query(q, rule, source, offsets, idx, diags);
        }

        Expr::InSubquery {
            expr: inner,
            subquery,
            ..
        } => {
            check_expr(inner, rule, source, offsets, idx, diags);
            check_query(subquery, rule, source, offsets, idx, diags);
        }

        Expr::Exists { subquery, .. } => {
            check_query(subquery, rule, source, offsets, idx, diags);
        }

        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                check_expr(op, rule, source, offsets, idx, diags);
            }
            for cond in conditions {
                check_expr(cond, rule, source, offsets, idx, diags);
            }
            for res in results {
                check_expr(res, rule, source, offsets, idx, diags);
            }
            if let Some(else_r) = else_result {
                check_expr(else_r, rule, source, offsets, idx, diags);
            }
        }

        _ => {}
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// Collect byte offsets of every `IN` keyword (case-insensitive, word-boundary,
/// outside strings/comments) in source order.
///
/// We must exclude `IN` that is part of `NOT IN` — we skip those because the
/// AST's `negated` flag handles them, and we don't want to consume an offset
/// slot that the AST will never fire on.  We therefore collect ALL `IN`
/// occurrences (including those following `NOT`) and let the AST traversal
/// consume or skip them in lock-step.
fn collect_in_offsets(source: &str) -> Vec<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);
    let kw = b"IN";
    let kw_len = kw.len();
    let mut offsets = Vec::new();

    let mut i = 0;
    while i + kw_len <= len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }

        // Word boundary before.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive match for "IN".
        let matches = bytes[i] == b'I' || bytes[i] == b'i';
        let matches = matches && (bytes[i + 1] == b'N' || bytes[i + 1] == b'n');

        if matches {
            // Word boundary after.
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);
            let all_code = (i..i + kw_len).all(|k| skip_map.is_code(k));

            if after_ok && all_code {
                offsets.push(i);
                i += kw_len;
                continue;
            }
        }

        i += 1;
    }

    offsets
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
