use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{BinaryOperator, Expr, Query, Select, SelectItem, SetExpr, Statement,
    TableFactor, Value};

pub struct DivisionByZero;

impl Rule for DivisionByZero {
    fn name(&self) -> &'static str {
        "Ambiguous/DivisionByZero"
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
            // Check any ON condition in the join.
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

/// Returns `true` when `expr` is a numeric literal whose value is zero.
fn is_zero_literal(expr: &Expr) -> bool {
    if let Expr::Value(Value::Number(s, _)) = expr {
        // Parse as f64 to handle 0, 0.0, 0.00, etc.
        s.parse::<f64>().map(|v| v == 0.0).unwrap_or(false)
    } else {
        false
    }
}

fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::BinaryOp { left, op, right } => {
            // Recurse into children first so nested / 0 inside `a / 2 / 0` is caught.
            check_expr(left, ctx, diags);
            check_expr(right, ctx, diags);

            if matches!(op, BinaryOperator::Divide) && is_zero_literal(right) {
                let (line, col) = find_division_position(&ctx.source);
                diags.push(Diagnostic {
                    rule: "Ambiguous/DivisionByZero",
                    message: "Division by zero literal; this will cause an error or return NULL"
                        .to_string(),
                    line,
                    col,
                });
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

/// Finds the first `/ 0` or `/ 0.0` (etc.) pattern in `source` and returns
/// its 1-indexed (line, col). Falls back to (1, 1) if not found.
///
/// Scans for a `/` character followed by optional whitespace followed by a
/// zero-value numeric token (`0`, `0.0`, `0.00`, etc.).
fn find_division_position(source: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'/' {
            // Skip whitespace after the slash.
            let mut j = i + 1;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            // Check if what follows is a zero literal.
            if j < len && bytes[j].is_ascii_digit() {
                let start = j;
                // Collect the numeric token.
                while j < len && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
                    j += 1;
                }
                let token = std::str::from_utf8(&bytes[start..j]).unwrap_or("");
                if token.parse::<f64>().map(|v| v == 0.0).unwrap_or(false) {
                    return offset_to_line_col(source, i);
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
