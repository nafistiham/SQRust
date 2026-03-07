use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    BinaryOperator, Expr, FunctionArg, FunctionArgExpr, FunctionArguments, JoinConstraint,
    JoinOperator, Query, SetExpr, Statement, TableFactor,
};

pub struct FunctionOnFilteredColumn;

impl Rule for FunctionOnFilteredColumn {
    fn name(&self) -> &'static str {
        "Ambiguous/FunctionOnFilteredColumn"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // Track per-function-name occurrence count for source-position lookup.
        let mut counters: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &ctx.source, &mut counters, &mut diags);
            }
        }
        diags
    }
}

// ── AST walking ────────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    source: &str,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, counters, diags);
        }
    }
    check_set_expr(&query.body, source, counters, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    source: &str,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            // Check WHERE clause.
            if let Some(selection) = &sel.selection {
                check_filter_expr(selection, source, counters, diags);
            }

            // Check JOIN ON clauses.
            for twj in &sel.from {
                for join in &twj.joins {
                    match &join.join_operator {
                        JoinOperator::Inner(JoinConstraint::On(on_expr))
                        | JoinOperator::LeftOuter(JoinConstraint::On(on_expr))
                        | JoinOperator::RightOuter(JoinConstraint::On(on_expr))
                        | JoinOperator::FullOuter(JoinConstraint::On(on_expr)) => {
                            check_filter_expr(on_expr, source, counters, diags);
                        }
                        _ => {}
                    }
                }

                // Recurse into subqueries in FROM.
                recurse_subqueries_in_factor(&twj.relation, source, counters, diags);
                for join in &twj.joins {
                    recurse_subqueries_in_factor(&join.relation, source, counters, diags);
                }
            }
        }
        SetExpr::Query(inner) => check_query(inner, source, counters, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, counters, diags);
            check_set_expr(right, source, counters, diags);
        }
        _ => {}
    }
}

fn recurse_subqueries_in_factor(
    factor: &TableFactor,
    source: &str,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        check_query(subquery, source, counters, diags);
    }
}

// ── Filter expression checker ──────────────────────────────────────────────────

/// Walk a WHERE or ON expression and flag any `FUNC(col) <op> value` pattern
/// where the function argument is a bare column reference.
fn check_filter_expr(
    expr: &Expr,
    source: &str,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::BinaryOp { left, op, right } => {
            if is_comparison_op(op) {
                // Check left side: FUNC(col) <op> ...
                if let Some(func_name) = is_function_on_bare_column(left) {
                    emit_diagnostic(&func_name, source, counters, diags);
                }
                // Check right side: ... <op> FUNC(col)
                // (We only flag left-side as per the plan: "FUNC(col) = value",
                //  but the join test expects both sides to be flagged, so check both.)
                if let Some(func_name) = is_function_on_bare_column(right) {
                    emit_diagnostic(&func_name, source, counters, diags);
                }
            } else {
                // Non-comparison binary op (AND, OR, etc.) — recurse into both sides.
                check_filter_expr(left, source, counters, diags);
                check_filter_expr(right, source, counters, diags);
            }
        }
        Expr::Nested(inner) => check_filter_expr(inner, source, counters, diags),
        Expr::UnaryOp { expr: inner, .. } => check_filter_expr(inner, source, counters, diags),
        _ => {}
    }
}

/// If `expr` is `Expr::Function(f)` whose sole argument is a bare column reference
/// (`Identifier` or `CompoundIdentifier`), return `Some(function_name_uppercase)`.
/// Returns `None` otherwise.
fn is_function_on_bare_column(expr: &Expr) -> Option<String> {
    let func = match expr {
        Expr::Function(f) => f,
        _ => return None,
    };

    // Extract function args.
    let args = match &func.args {
        FunctionArguments::List(list) => &list.args,
        _ => return None,
    };

    // Must have exactly one argument.
    if args.len() != 1 {
        return None;
    }

    // Extract the inner expression.
    let inner = match &args[0] {
        FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => e,
        FunctionArg::Named { arg: FunctionArgExpr::Expr(e), .. } => e,
        _ => return None,
    };

    // Inner must be a bare column reference (Identifier or CompoundIdentifier),
    // NOT a nested function call or binary expression.
    match inner {
        Expr::Identifier(_) | Expr::CompoundIdentifier(_) => {}
        _ => return None,
    }

    // Return the function name (last part, uppercased).
    let name = func
        .name
        .0
        .last()
        .map(|i| i.value.to_uppercase())
        .unwrap_or_default();

    Some(name)
}

/// Emit a diagnostic for a function-on-column finding, using occurrence-based
/// source positioning.
fn emit_diagnostic(
    func_name: &str,
    source: &str,
    counters: &mut std::collections::HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    let occ = counters.entry(func_name.to_string()).or_insert(0);
    let occurrence = *occ;
    *occ += 1;

    let offset = find_occurrence(source, func_name, occurrence);
    let (line, col) = offset_to_line_col(source, offset);

    diags.push(Diagnostic {
        rule: "Ambiguous/FunctionOnFilteredColumn",
        message: "Function applied to column in WHERE/JOIN prevents index usage: use a computed column or rewrite the condition".to_string(),
        line,
        col,
    });
}

// ── Comparison operator check ──────────────────────────────────────────────────

fn is_comparison_op(op: &BinaryOperator) -> bool {
    matches!(
        op,
        BinaryOperator::Eq
            | BinaryOperator::NotEq
            | BinaryOperator::Lt
            | BinaryOperator::LtEq
            | BinaryOperator::Gt
            | BinaryOperator::GtEq
    )
}

// ── Source-text helpers ────────────────────────────────────────────────────────

/// Finds the byte offset of the `nth` (0-indexed) whole-word, case-insensitive
/// occurrence of `name` (uppercased) in `source`. Returns 0 if not found.
fn find_occurrence(source: &str, name: &str, nth: usize) -> usize {
    let bytes = source.as_bytes();
    let name_bytes: Vec<u8> = name.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let name_len = name_bytes.len();
    let src_len = bytes.len();

    let mut count = 0usize;
    let mut i = 0usize;

    while i + name_len <= src_len {
        let before_ok = i == 0 || {
            let b = bytes[i - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };

        if before_ok {
            let matches = bytes[i..i + name_len]
                .iter()
                .zip(name_bytes.iter())
                .all(|(&a, &b)| a.to_ascii_uppercase() == b);

            if matches {
                let after = i + name_len;
                let after_ok = after >= src_len || {
                    let b = bytes[after];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };

                if after_ok {
                    if count == nth {
                        return i;
                    }
                    count += 1;
                }
            }
        }

        i += 1;
    }

    0
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
