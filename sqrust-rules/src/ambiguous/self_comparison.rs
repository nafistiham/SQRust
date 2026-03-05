use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{BinaryOperator, Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct SelfComparison;

impl Rule for SelfComparison {
    fn name(&self) -> &'static str {
        "Ambiguous/SelfComparison"
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

/// Returns true when the operator is a comparison that would be trivially
/// redundant when both operands are the same.
fn is_comparison_op(op: &BinaryOperator) -> bool {
    matches!(
        op,
        BinaryOperator::Eq
            | BinaryOperator::NotEq
            | BinaryOperator::Lt
            | BinaryOperator::Gt
            | BinaryOperator::LtEq
            | BinaryOperator::GtEq
    )
}

/// Strips `Nested` wrappers so that `(col)` resolves to the inner identifier.
fn unwrap_nested(expr: &Expr) -> &Expr {
    let mut current = expr;
    while let Expr::Nested(inner) = current {
        current = inner;
    }
    current
}

/// Case-insensitive equality check for identifier-like expressions.
/// Both sides must be plain or compound identifiers. Returns the display name
/// when they are equal, `None` otherwise.
fn self_comparison_name<'a>(left: &'a Expr, right: &'a Expr) -> Option<String> {
    let l = unwrap_nested(left);
    let r = unwrap_nested(right);

    match (l, r) {
        (Expr::Identifier(li), Expr::Identifier(ri)) => {
            if li.value.to_lowercase() == ri.value.to_lowercase() {
                Some(li.value.clone())
            } else {
                None
            }
        }
        (Expr::CompoundIdentifier(lparts), Expr::CompoundIdentifier(rparts)) => {
            if lparts.len() == rparts.len()
                && lparts
                    .iter()
                    .zip(rparts.iter())
                    .all(|(a, b)| a.value.to_lowercase() == b.value.to_lowercase())
            {
                let name = lparts
                    .iter()
                    .map(|i| i.value.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                Some(name)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn check_expr(expr: &Expr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::BinaryOp { left, op, right } => {
            // Recurse into children first.
            check_expr(left, ctx, diags);
            check_expr(right, ctx, diags);

            // Then check whether this node is a self-comparison.
            if is_comparison_op(op) {
                if let Some(name) = self_comparison_name(left, right) {
                    let (line, col) = find_identifier_position(&ctx.source, &name);
                    diags.push(Diagnostic {
                        rule: "Ambiguous/SelfComparison",
                        message: format!(
                            "Expression compares '{}' to itself; this is always TRUE or NULL",
                            name
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
        Expr::Between { expr: inner, low, high, .. } => {
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

/// Finds the first word-boundary occurrence of `name` (case-insensitive) in
/// `source` and returns a 1-indexed (line, col). Falls back to (1, 1).
fn find_identifier_position(source: &str, name: &str) -> (usize, usize) {
    let upper = source.to_uppercase();
    let name_upper = name.to_uppercase();
    let bytes = upper.as_bytes();
    let name_bytes = name_upper.as_bytes();
    let name_len = name_bytes.len();

    let mut i = 0;
    while i + name_len <= bytes.len() {
        if bytes[i..i + name_len] == *name_bytes {
            let before_ok =
                i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let after = i + name_len;
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
