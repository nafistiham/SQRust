use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{BinaryOperator, Expr, Query, Select, SelectItem, SetExpr, Statement,
    TableFactor, Value};

pub struct IntegerDivision;

impl Rule for IntegerDivision {
    fn name(&self) -> &'static str {
        "Ambiguous/IntegerDivision"
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
    // Check SELECT projection.
    for item in &select.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, ctx, diags);
        }
    }

    // Check FROM subqueries.
    for twj in &select.from {
        collect_from_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            collect_from_table_factor(&join.relation, ctx, diags);
            if let sqlparser::ast::JoinOperator::Inner(sqlparser::ast::JoinConstraint::On(e))
            | sqlparser::ast::JoinOperator::LeftOuter(sqlparser::ast::JoinConstraint::On(e))
            | sqlparser::ast::JoinOperator::RightOuter(sqlparser::ast::JoinConstraint::On(e))
            | sqlparser::ast::JoinOperator::FullOuter(sqlparser::ast::JoinConstraint::On(e)) =
                &join.join_operator
            {
                check_expr(e, ctx, diags);
            }
        }
    }

    // Check WHERE.
    if let Some(selection) = &select.selection {
        check_expr(selection, ctx, diags);
    }

    // Check HAVING.
    if let Some(having) = &select.having {
        check_expr(having, ctx, diags);
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

/// Returns `true` when `expr` is an integer literal (no decimal point).
fn is_integer_literal(expr: &Expr) -> Option<String> {
    if let Expr::Value(Value::Number(s, _)) = expr {
        if !s.contains('.') {
            return Some(s.clone());
        }
    }
    None
}

fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::BinaryOp { left, op, right } => {
            // Recurse into children first so nested divisions are caught.
            check_expr(left, ctx, diags);
            check_expr(right, ctx, diags);

            if matches!(op, BinaryOperator::Divide) {
                if let (Some(lval), Some(rval)) =
                    (is_integer_literal(left), is_integer_literal(right))
                {
                    let (line, col) = find_integer_division_position(&ctx.source, &lval, &rval);
                    diags.push(Diagnostic {
                        rule: "Ambiguous/IntegerDivision",
                        message: format!(
                            "Integer division {} / {} truncates towards zero \
                             — use CAST(expr AS FLOAT) or add .0 to a literal for decimal division",
                            lval, rval
                        ),
                        line,
                        col,
                    });
                }
            }
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, ctx, diags);
        }
        Expr::Nested(inner) => {
            check_expr(inner, ctx, diags);
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
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
        Expr::Between {
            expr: inner,
            low,
            high,
            ..
        } => {
            check_expr(inner, ctx, diags);
            check_expr(low, ctx, diags);
            check_expr(high, ctx, diags);
        }
        Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
            check_expr(inner, ctx, diags);
        }
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, ctx, diags);
        }
        _ => {}
    }
}

/// Finds the position of an integer/integer division pattern (e.g. `1/2`) in source.
/// Searches for `lval` followed by optional spaces, `/`, optional spaces, `rval`.
/// Falls back to (1, 1) if not found.
fn find_integer_division_position(source: &str, lval: &str, rval: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let lval_bytes = lval.as_bytes();
    let rval_bytes = rval.as_bytes();
    let llen = lval_bytes.len();
    let rlen = rval_bytes.len();

    let mut i = 0;
    while i + llen <= len {
        // Match lval at position i — check word boundary after
        if &bytes[i..i + llen] == lval_bytes {
            let after_l = i + llen;
            // Skip whitespace
            let mut j = after_l;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j < len && bytes[j] == b'/' {
                let slash_pos = j;
                j += 1;
                // Skip whitespace after /
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                // Match rval
                if j + rlen <= len && &bytes[j..j + rlen] == rval_bytes {
                    // Ensure the next char after rval is not a digit or dot (not part of a larger number)
                    let after_r = j + rlen;
                    let rval_ends =
                        after_r >= len || (!bytes[after_r].is_ascii_digit() && bytes[after_r] != b'.');
                    if rval_ends {
                        return offset_to_line_col(source, slash_pos);
                    }
                }
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
