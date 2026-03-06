use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, Value};

pub struct NullInNotIn;

impl Rule for NullInNotIn {
    fn name(&self) -> &'static str {
        "Lint/NullInNotIn"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST may be incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // Track occurrence count so we can locate the Nth `NOT IN` phrase in source.
        let mut occurrence: usize = 0;

        for stmt in &ctx.statements {
            check_statement(stmt, &mut diags, &ctx.source, &mut occurrence);
        }

        diags
    }
}

// ── Statement-level walker ────────────────────────────────────────────────────

fn check_statement(stmt: &Statement, diags: &mut Vec<Diagnostic>, source: &str, occ: &mut usize) {
    match stmt {
        Statement::Query(q) => check_query(q, diags, source, occ),
        Statement::Insert(insert) => {
            if let Some(src) = &insert.source {
                check_query(src, diags, source, occ);
            }
        }
        Statement::Update { selection, .. } => {
            if let Some(expr) = selection {
                check_expr(expr, diags, source, occ);
            }
        }
        Statement::Delete(delete) => {
            if let Some(expr) = &delete.selection {
                check_expr(expr, diags, source, occ);
            }
        }
        _ => {}
    }
}

fn check_query(query: &Query, diags: &mut Vec<Diagnostic>, source: &str, occ: &mut usize) {
    match query.body.as_ref() {
        SetExpr::Select(select) => check_select(select, diags, source, occ),
        SetExpr::Query(q) => check_query(q, diags, source, occ),
        SetExpr::SetOperation { left, right, .. } => {
            match left.as_ref() {
                SetExpr::Select(s) => check_select(s, diags, source, occ),
                SetExpr::Query(q) => check_query(q, diags, source, occ),
                _ => {}
            }
            match right.as_ref() {
                SetExpr::Select(s) => check_select(s, diags, source, occ),
                SetExpr::Query(q) => check_query(q, diags, source, occ),
                _ => {}
            }
        }
        _ => {}
    }
}

fn check_select(select: &Select, diags: &mut Vec<Diagnostic>, source: &str, occ: &mut usize) {
    // Projection expressions
    for item in &select.projection {
        if let SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } = item {
            check_expr(expr, diags, source, occ);
        }
    }

    // WHERE clause
    if let Some(expr) = &select.selection {
        check_expr(expr, diags, source, occ);
    }

    // HAVING clause
    if let Some(expr) = &select.having {
        check_expr(expr, diags, source, occ);
    }
}

// ── Expression walker ─────────────────────────────────────────────────────────

fn check_expr(expr: &Expr, diags: &mut Vec<Diagnostic>, source: &str, occ: &mut usize) {
    match expr {
        // NOT IN (...) — the case we want to flag when list contains NULL
        Expr::InList {
            expr: inner,
            list,
            negated: true,
        } => {
            let has_null = list.iter().any(|e| matches!(e, Expr::Value(Value::Null)));
            if has_null {
                // Find the Nth occurrence of "NOT IN" in source (case-insensitive).
                let (line, col) = find_nth_phrase(source, "NOT IN", *occ);
                *occ += 1;
                diags.push(Diagnostic {
                    rule: "Lint/NullInNotIn",
                    message:
                        "NOT IN list contains NULL; this will always produce an empty result set"
                            .to_string(),
                    line,
                    col,
                });
            }
            // Recurse into inner and list elements.
            check_expr(inner, diags, source, occ);
            for e in list {
                check_expr(e, diags, source, occ);
            }
        }

        // Positive IN (...) — not flagged, but recurse in case list has subqueries
        Expr::InList {
            expr: inner,
            list,
            negated: false,
        } => {
            check_expr(inner, diags, source, occ);
            for e in list {
                check_expr(e, diags, source, occ);
            }
        }

        // Binary operators — recurse both sides
        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, diags, source, occ);
            check_expr(right, diags, source, occ);
        }

        // Parenthesised expression
        Expr::Nested(inner) => check_expr(inner, diags, source, occ),

        // Subquery (scalar)
        Expr::Subquery(q) => check_query(q, diags, source, occ),

        // [NOT] IN (SELECT ...) — recurse into the inner expression and the subquery
        Expr::InSubquery {
            expr: inner,
            subquery,
            ..
        } => {
            check_expr(inner, diags, source, occ);
            check_query(subquery, diags, source, occ);
        }

        // EXISTS (SELECT ...) — recurse into the subquery
        Expr::Exists { subquery, .. } => check_query(subquery, diags, source, occ),

        // Function calls — check arguments
        Expr::Function(f) => {
            use sqlparser::ast::FunctionArguments;
            if let FunctionArguments::List(arg_list) = &f.args {
                for arg in &arg_list.args {
                    if let sqlparser::ast::FunctionArg::Unnamed(
                        sqlparser::ast::FunctionArgExpr::Expr(e),
                    ) = arg
                    {
                        check_expr(e, diags, source, occ);
                    }
                }
            }
        }

        // CASE WHEN expressions
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                check_expr(op, diags, source, occ);
            }
            for cond in conditions {
                check_expr(cond, diags, source, occ);
            }
            for res in results {
                check_expr(res, diags, source, occ);
            }
            if let Some(el) = else_result {
                check_expr(el, diags, source, occ);
            }
        }

        // Unary operators
        Expr::UnaryOp { expr: inner, .. } => check_expr(inner, diags, source, occ),

        // IS NULL / IS NOT NULL
        Expr::IsNull(inner) | Expr::IsNotNull(inner) => check_expr(inner, diags, source, occ),

        // BETWEEN
        Expr::Between {
            expr: inner,
            low,
            high,
            ..
        } => {
            check_expr(inner, diags, source, occ);
            check_expr(low, diags, source, occ);
            check_expr(high, diags, source, occ);
        }

        // LIKE, ILIKE
        Expr::Like {
            expr: inner,
            pattern,
            ..
        }
        | Expr::ILike {
            expr: inner,
            pattern,
            ..
        } => {
            check_expr(inner, diags, source, occ);
            check_expr(pattern, diags, source, occ);
        }

        // Everything else (literals, identifiers, etc.) — nothing to recurse into
        _ => {}
    }
}

// ── Source-text helpers ───────────────────────────────────────────────────────

/// Finds the (line, col) of the `nth` (0-indexed) whole-word, case-insensitive
/// occurrence of `phrase` in `source`. Returns (1, 1) if not found.
fn find_nth_phrase(source: &str, phrase: &str, nth: usize) -> (usize, usize) {
    let phrase_upper = phrase.to_uppercase();
    let source_upper = source.to_uppercase();
    let phrase_bytes = phrase_upper.as_bytes();
    let src_bytes = source_upper.as_bytes();
    let phrase_len = phrase_bytes.len();
    let src_len = src_bytes.len();

    let mut count = 0usize;
    let mut i = 0usize;

    while i + phrase_len <= src_len {
        // Check if source_upper[i..i+phrase_len] == phrase_upper
        if src_bytes[i..i + phrase_len] == *phrase_bytes {
            // Word boundary before
            let before_ok = i == 0 || {
                let b = src_bytes[i - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            // Word boundary after
            let after = i + phrase_len;
            let after_ok = after >= src_len || {
                let b = src_bytes[after];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if before_ok && after_ok {
                if count == nth {
                    return offset_to_line_col(source, i);
                }
                count += 1;
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
