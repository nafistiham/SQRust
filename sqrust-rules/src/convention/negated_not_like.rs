use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor, UnaryOperator,
};

pub struct NegatedNotLike;

impl Rule for NegatedNotLike {
    fn name(&self) -> &'static str {
        "Convention/NegatedNotLike"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            check_statement(stmt, ctx, &mut diags);
        }
        diags
    }
}

fn check_statement(stmt: &Statement, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match stmt {
        Statement::Query(q) => check_query(q, ctx, diags),
        Statement::Insert(insert) => {
            if let Some(src) = &insert.source {
                check_query(src, ctx, diags);
            }
        }
        Statement::Update { selection, assignments, .. } => {
            if let Some(expr) = selection {
                check_expr(expr, ctx, diags);
            }
            for assignment in assignments {
                check_expr(&assignment.value, ctx, diags);
            }
        }
        Statement::Delete(delete) => {
            if let Some(expr) = &delete.selection {
                check_expr(expr, ctx, diags);
            }
        }
        _ => {}
    }
}

fn check_query(q: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &q.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, diags);
        }
    }
    check_set_expr(&q.body, ctx, diags);
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, ctx, diags),
        SetExpr::Query(q) => check_query(q, ctx, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, diags);
            check_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(sel: &Select, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Check projection.
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                check_expr(e, ctx, diags);
            }
            _ => {}
        }
    }

    // Check FROM subqueries.
    for twj in &sel.from {
        check_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, ctx, diags);
        }
    }

    // Check WHERE.
    if let Some(selection) = &sel.selection {
        check_expr(selection, ctx, diags);
    }

    // Check HAVING.
    if let Some(having) = &sel.having {
        check_expr(having, ctx, diags);
    }
}

fn check_table_factor(factor: &TableFactor, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = factor {
        check_query(subquery, ctx, diags);
    }
}

/// Returns the diagnostic message to use when `NOT` wraps `inner`, or `None`
/// if `inner` is not one of the targeted predicate types.
fn negated_predicate_message(inner: &Expr) -> Option<&'static str> {
    match inner {
        Expr::Like { .. } | Expr::ILike { .. } => {
            Some("Prefer 'col NOT LIKE pattern' over 'NOT col LIKE pattern' for negated predicates")
        }
        Expr::Between { .. } => {
            Some("Prefer 'col NOT BETWEEN a AND b' over 'NOT col BETWEEN a AND b' for negated predicates")
        }
        Expr::InList { .. } | Expr::InSubquery { .. } => {
            Some("Prefer 'col NOT IN (...)' over 'NOT col IN (...)' for negated predicates")
        }
        _ => None,
    }
}

fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::UnaryOp { op: UnaryOperator::Not, expr: inner } => {
            // Check if the direct inner expression is a flagged predicate.
            // Note: NOT (expr) produces Nested as the inner node — we do NOT unwrap
            // Nested here, so `NOT (col LIKE x)` is not flagged (parens change form).
            if let Some(msg) = negated_predicate_message(inner) {
                let (line, col) = find_not_keyword_position(&ctx.source, diags.len());
                diags.push(Diagnostic {
                    rule: "Convention/NegatedNotLike",
                    message: msg.to_string(),
                    line,
                    col,
                });
            }
            // Recurse into the inner expression regardless.
            check_expr(inner, ctx, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, ctx, diags);
        }
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, ctx, diags);
            check_expr(right, ctx, diags);
        }
        Expr::Nested(inner) => {
            check_expr(inner, ctx, diags);
        }
        Expr::Case { operand, conditions, results, else_result } => {
            if let Some(op) = operand {
                check_expr(op, ctx, diags);
            }
            for cond in conditions {
                check_expr(cond, ctx, diags);
            }
            for result in results {
                check_expr(result, ctx, diags);
            }
            if let Some(else_e) = else_result {
                check_expr(else_e, ctx, diags);
            }
        }
        Expr::InList { expr: inner, list, .. } => {
            check_expr(inner, ctx, diags);
            for e in list {
                check_expr(e, ctx, diags);
            }
        }
        Expr::InSubquery { expr: inner, subquery, .. } => {
            check_expr(inner, ctx, diags);
            check_query(subquery, ctx, diags);
        }
        Expr::Between { expr: inner, low, high, .. } => {
            check_expr(inner, ctx, diags);
            check_expr(low, ctx, diags);
            check_expr(high, ctx, diags);
        }
        Expr::Like { expr: inner, pattern, .. } | Expr::ILike { expr: inner, pattern, .. } => {
            check_expr(inner, ctx, diags);
            check_expr(pattern, ctx, diags);
        }
        Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
            check_expr(inner, ctx, diags);
        }
        Expr::Subquery(q) | Expr::Exists { subquery: q, .. } => {
            check_query(q, ctx, diags);
        }
        Expr::Function(f) => {
            use sqlparser::ast::{FunctionArg, FunctionArgExpr, FunctionArguments};
            if let FunctionArguments::List(list) = &f.args {
                for arg in &list.args {
                    if let FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) = arg {
                        check_expr(e, ctx, diags);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Finds the byte offset of the Nth occurrence of the word `NOT` (case-insensitive,
/// word-boundary) in `source`. The `nth` parameter is the count of violations already
/// recorded, used to locate the correct occurrence in the source text.
///
/// Falls back to (1, 1) if the position cannot be determined.
fn find_not_keyword_position(source: &str, nth: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut count = 0usize;
    let mut i = 0;

    while i + 3 <= len {
        let before_ok = i == 0 || !is_word_char_byte(bytes[i - 1]);
        if before_ok && bytes[i..i + 3].eq_ignore_ascii_case(b"NOT") {
            let after_ok = i + 3 >= len || !is_word_char_byte(bytes[i + 3]);
            if after_ok {
                if count == nth {
                    return offset_to_line_col(source, i);
                }
                count += 1;
                i += 3;
                continue;
            }
        }
        i += 1;
    }

    (1, 1)
}

#[inline]
fn is_word_char_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
