use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, FunctionArg, FunctionArgExpr, FunctionArguments, GroupByExpr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct UnusedJoin;

impl Rule for UnusedJoin {
    fn name(&self) -> &'static str {
        "Structure/UnusedJoin"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, ctx, &mut diags);
            }
        }
        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(q: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &q.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, diags);
        }
    }

    // Collect ORDER BY qualifiers from this query level.
    let mut order_qualifiers: std::collections::HashSet<String> = std::collections::HashSet::new();
    if let Some(order_by) = &q.order_by {
        for ob_expr in &order_by.exprs {
            collect_qualifiers_in_expr(&ob_expr.expr, &mut order_qualifiers);
        }
    }

    check_set_expr(&q.body, ctx, &order_qualifiers, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    ctx: &FileContext,
    order_qualifiers: &std::collections::HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, ctx, order_qualifiers, diags),
        SetExpr::Query(inner) => check_query(inner, ctx, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, order_qualifiers, diags);
            check_set_expr(right, ctx, order_qualifiers, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    ctx: &FileContext,
    extra_qualifiers: &std::collections::HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    // Collect qualifiers used in SELECT projection, WHERE, HAVING, GROUP BY.
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();

    // SELECT projection.
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                collect_qualifiers_in_expr(e, &mut used);
            }
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => {}
        }
    }

    // WHERE.
    if let Some(selection) = &sel.selection {
        collect_qualifiers_in_expr(selection, &mut used);
    }

    // HAVING.
    if let Some(having) = &sel.having {
        collect_qualifiers_in_expr(having, &mut used);
    }

    // GROUP BY.
    if let GroupByExpr::Expressions(exprs, _) = &sel.group_by {
        for e in exprs {
            collect_qualifiers_in_expr(e, &mut used);
        }
    }

    // Add any extra qualifiers (e.g. from ORDER BY).
    for q in extra_qualifiers {
        used.insert(q.clone());
    }

    // Check each JOIN.
    for twj in &sel.from {
        for join in &twj.joins {
            if let Some(ref_name) = table_factor_ref_name(&join.relation) {
                let ref_upper = ref_name.to_uppercase();
                if !used.contains(&ref_upper) {
                    // Not used in SELECT/WHERE/HAVING/GROUP BY/ORDER BY.
                    let source = &ctx.source;
                    let def_pos = find_word_position(source, &ref_name).unwrap_or(0);
                    let (line, col) = offset_to_line_col(source, def_pos);
                    diags.push(Diagnostic {
                        rule: "Structure/UnusedJoin",
                        message: format!(
                            "JOIN table '{}' is never referenced in query output; \
                             the join may be unnecessary",
                            ref_name
                        ),
                        line,
                        col,
                    });
                }
            }

            // Recurse into derived subqueries inside the JOIN.
            recurse_table_factor(&join.relation, ctx, diags);
        }

        // Recurse into derived subqueries in the primary FROM table.
        recurse_table_factor(&twj.relation, ctx, diags);
    }
}

fn recurse_table_factor(tf: &TableFactor, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, ctx, diags);
    }
}

// ── Qualifier collection ───────────────────────────────────────────────────────

/// Collect all `qualifier` parts from `table.column` compound identifiers.
/// E.g. `b.name` contributes `"B"` to the set.
fn collect_qualifiers_in_expr(expr: &Expr, qualifiers: &mut std::collections::HashSet<String>) {
    match expr {
        Expr::CompoundIdentifier(parts) if parts.len() >= 2 => {
            // First part is the table/alias qualifier.
            qualifiers.insert(parts[0].value.to_uppercase());
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_qualifiers_in_expr(left, qualifiers);
            collect_qualifiers_in_expr(right, qualifiers);
        }
        Expr::UnaryOp { expr: inner, .. } => collect_qualifiers_in_expr(inner, qualifiers),
        Expr::Nested(inner) => collect_qualifiers_in_expr(inner, qualifiers),
        Expr::Cast { expr: inner, .. } => collect_qualifiers_in_expr(inner, qualifiers),
        Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
            collect_qualifiers_in_expr(inner, qualifiers)
        }
        Expr::Between {
            expr: e, low, high, ..
        } => {
            collect_qualifiers_in_expr(e, qualifiers);
            collect_qualifiers_in_expr(low, qualifiers);
            collect_qualifiers_in_expr(high, qualifiers);
        }
        Expr::InList { expr: inner, list, .. } => {
            collect_qualifiers_in_expr(inner, qualifiers);
            for e in list {
                collect_qualifiers_in_expr(e, qualifiers);
            }
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                collect_qualifiers_in_expr(op, qualifiers);
            }
            for cond in conditions {
                collect_qualifiers_in_expr(cond, qualifiers);
            }
            for res in results {
                collect_qualifiers_in_expr(res, qualifiers);
            }
            if let Some(else_e) = else_result {
                collect_qualifiers_in_expr(else_e, qualifiers);
            }
        }
        Expr::Function(f) => {
            if let FunctionArguments::List(arg_list) = &f.args {
                for arg in &arg_list.args {
                    if let FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) = arg {
                        collect_qualifiers_in_expr(e, qualifiers);
                    }
                }
            }
        }
        Expr::Subquery(_) | Expr::InSubquery { .. } | Expr::Exists { .. } => {
            // Don't recurse into subqueries — they have their own scope.
        }
        _ => {}
    }
}

// ── AST helpers ───────────────────────────────────────────────────────────────

fn table_factor_ref_name(tf: &TableFactor) -> Option<String> {
    match tf {
        TableFactor::Table { name, alias, .. } => {
            if let Some(a) = alias {
                Some(a.name.value.clone())
            } else {
                name.0.last().map(|ident| ident.value.clone())
            }
        }
        TableFactor::Derived { alias, .. } => alias.as_ref().map(|a| a.name.value.clone()),
        _ => None,
    }
}

// ── Source-text helpers ───────────────────────────────────────────────────────

fn find_word_position(source: &str, word: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let word_upper: Vec<u8> = word.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let wlen = word_upper.len();
    let len = bytes.len();
    let skip = SkipMap::build(source);
    let mut i = 0;
    while i + wlen <= len {
        if !skip.is_code(i) {
            i += 1;
            continue;
        }
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }
        let matches = bytes[i..i + wlen]
            .iter()
            .zip(word_upper.iter())
            .all(|(&a, &b)| a.to_ascii_uppercase() == b);
        if matches {
            let end = i + wlen;
            let after_ok = end >= len || !is_word_char(bytes[end]);
            if after_ok {
                return Some(i);
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
