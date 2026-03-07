use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};
use sqlparser::ast::Value;

pub struct UnnecessaryCaseWhen;

/// Returns `true` if the expression is a boolean-true or integer-1 literal.
fn is_true_literal(e: &Expr) -> bool {
    match e {
        Expr::Value(Value::Boolean(true)) => true,
        Expr::Value(Value::Number(n, _)) => n == "1",
        _ => false,
    }
}

/// Returns `true` if the expression is a boolean-false or integer-0 literal.
fn is_false_literal(e: &Expr) -> bool {
    match e {
        Expr::Value(Value::Boolean(false)) => true,
        Expr::Value(Value::Number(n, _)) => n == "0",
        _ => false,
    }
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Returns `true` if `ch` is a SQL word character.
#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Finds the byte offset of the Nth occurrence (1-indexed) of `CASE` keyword
/// (case-insensitive, word-boundary) in `source`.
fn find_nth_case_offset(source: &str, n: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let kw = b"CASE";
    let kw_len = kw.len();
    let mut count = 0;
    let mut i = 0;

    while i + kw_len <= len {
        // Case-insensitive match
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(kw.iter())
            .all(|(&a, &b)| a.eq_ignore_ascii_case(&b));

        if matches {
            // Word boundary before
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            // Word boundary after
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);

            if before_ok && after_ok {
                count += 1;
                if count == n {
                    return Some(i);
                }
                i += kw_len;
                continue;
            }
        }

        i += 1;
    }

    None
}

impl Rule for UnnecessaryCaseWhen {
    fn name(&self) -> &'static str {
        "Convention/UnnecessaryCaseWhen"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // We track the occurrence index of each CASE in source order so we can
        // map AST nodes back to source positions.
        let mut case_counter = 0usize;

        for stmt in &ctx.statements {
            collect_from_statement(stmt, ctx, &mut diags, &mut case_counter);
        }

        diags
    }
}

fn collect_from_statement(
    stmt: &Statement,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
    counter: &mut usize,
) {
    if let Statement::Query(query) = stmt {
        collect_from_query(query, ctx, diags, counter);
    }
}

fn collect_from_query(
    query: &Query,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
    counter: &mut usize,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, ctx, diags, counter);
        }
    }

    if let Some(order_by) = &query.order_by {
        for ob_expr in &order_by.exprs {
            check_expr(&ob_expr.expr, ctx, diags, counter);
        }
    }

    collect_from_set_expr(&query.body, ctx, diags, counter);
}

fn collect_from_set_expr(
    expr: &SetExpr,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
    counter: &mut usize,
) {
    match expr {
        SetExpr::Select(select) => collect_from_select(select, ctx, diags, counter),
        SetExpr::Query(inner) => collect_from_query(inner, ctx, diags, counter),
        SetExpr::SetOperation { left, right, .. } => {
            collect_from_set_expr(left, ctx, diags, counter);
            collect_from_set_expr(right, ctx, diags, counter);
        }
        _ => {}
    }
}

fn collect_from_select(
    select: &Select,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
    counter: &mut usize,
) {
    for item in &select.projection {
        if let SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } = item {
            check_expr(e, ctx, diags, counter);
        }
    }

    for table_with_joins in &select.from {
        collect_from_table_factor(&table_with_joins.relation, ctx, diags, counter);
        for join in &table_with_joins.joins {
            collect_from_table_factor(&join.relation, ctx, diags, counter);
        }
    }

    if let Some(selection) = &select.selection {
        check_expr(selection, ctx, diags, counter);
    }

    if let Some(having) = &select.having {
        check_expr(having, ctx, diags, counter);
    }
}

fn collect_from_table_factor(
    factor: &TableFactor,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
    counter: &mut usize,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        collect_from_query(subquery, ctx, diags, counter);
    }
}

fn check_expr(
    expr: &Expr,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
    counter: &mut usize,
) {
    match expr {
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            // Count this CASE occurrence (pre-order: count before recursing).
            *counter += 1;
            let this_occurrence = *counter;

            // Check if this is a searched CASE (no operand) with exactly one WHEN.
            if operand.is_none() && conditions.len() == 1 {
                if let Some(else_e) = else_result {
                    let then_e = &results[0];
                    let simplifiable = (is_true_literal(then_e) && is_false_literal(else_e))
                        || (is_false_literal(then_e) && is_true_literal(else_e));

                    if simplifiable {
                        let offset = find_nth_case_offset(&ctx.source, this_occurrence)
                            .unwrap_or(0);
                        let (line, col) = line_col(&ctx.source, offset);
                        diags.push(Diagnostic {
                            rule: self::rule_name(),
                            message: "CASE expression returns boolean literals and can be simplified"
                                .to_string(),
                            line,
                            col,
                        });
                    }
                }
            }

            // Recurse into operand.
            if let Some(op) = operand {
                check_expr(op, ctx, diags, counter);
            }

            // Recurse into WHEN conditions and THEN results.
            for cond in conditions {
                check_expr(cond, ctx, diags, counter);
            }
            for result in results {
                check_expr(result, ctx, diags, counter);
            }

            // Recurse into ELSE result.
            if let Some(else_e) = else_result {
                check_expr(else_e, ctx, diags, counter);
            }
        }

        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, ctx, diags, counter);
            check_expr(right, ctx, diags, counter);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, ctx, diags, counter);
        }
        Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
            check_expr(inner, ctx, diags, counter);
        }
        Expr::IsDistinctFrom(left, right) | Expr::IsNotDistinctFrom(left, right) => {
            check_expr(left, ctx, diags, counter);
            check_expr(right, ctx, diags, counter);
        }
        Expr::InList { expr: inner, list, .. } => {
            check_expr(inner, ctx, diags, counter);
            for e in list {
                check_expr(e, ctx, diags, counter);
            }
        }
        Expr::Between {
            expr: inner,
            low,
            high,
            ..
        } => {
            check_expr(inner, ctx, diags, counter);
            check_expr(low, ctx, diags, counter);
            check_expr(high, ctx, diags, counter);
        }
        Expr::Function(f) => {
            if let sqlparser::ast::FunctionArguments::List(arg_list) = &f.args {
                for arg in &arg_list.args {
                    if let sqlparser::ast::FunctionArg::Unnamed(
                        sqlparser::ast::FunctionArgExpr::Expr(e),
                    ) = arg
                    {
                        check_expr(e, ctx, diags, counter);
                    }
                }
            }
        }
        Expr::Cast { expr: inner, .. } => {
            check_expr(inner, ctx, diags, counter);
        }
        Expr::Nested(inner) => {
            check_expr(inner, ctx, diags, counter);
        }
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, ctx, diags, counter);
        }
        _ => {}
    }
}

/// Returns the rule name string (used inside `check_expr` where `self` is not available).
fn rule_name() -> &'static str {
    "Convention/UnnecessaryCaseWhen"
}
