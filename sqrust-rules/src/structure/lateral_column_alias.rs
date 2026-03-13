use std::collections::HashSet;

use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, GroupByExpr, Ident, Query, Select, SelectItem, SetExpr, Statement, TableFactor,
};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct LateralColumnAlias;

impl Rule for LateralColumnAlias {
    fn name(&self) -> &'static str {
        "Structure/LateralColumnAlias"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.name(), ctx, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, ctx, diags);
        }
    }

    check_set_expr(&query.body, rule, ctx, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, rule, ctx, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, rule, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, rule, ctx, diags);
            check_set_expr(right, rule, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Collect all aliases defined in the SELECT projection.
    let aliases: HashSet<String> = sel
        .projection
        .iter()
        .filter_map(|item| {
            if let SelectItem::ExprWithAlias { alias, .. } = item {
                Some(alias.value.to_lowercase())
            } else {
                None
            }
        })
        .collect();

    if aliases.is_empty() {
        // No aliases — nothing can be a lateral alias reference.
        // Still recurse into subqueries in FROM.
        recurse_from(sel, rule, ctx, diags);
        return;
    }

    // Check WHERE clause.
    if let Some(selection) = &sel.selection {
        collect_lateral_alias_refs(selection, &aliases, rule, ctx, diags);
    }

    // Check GROUP BY expressions.
    if let GroupByExpr::Expressions(exprs, _) = &sel.group_by {
        for expr in exprs {
            collect_lateral_alias_refs(expr, &aliases, rule, ctx, diags);
        }
    }

    // Check HAVING clause.
    if let Some(having) = &sel.having {
        collect_lateral_alias_refs(having, &aliases, rule, ctx, diags);
    }

    // Recurse into subqueries in the FROM clause.
    recurse_from(sel, rule, ctx, diags);
}

fn recurse_from(
    sel: &Select,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, rule, ctx, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, rule, ctx, diags);
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, rule, ctx, diags);
    }
}

// ── Lateral alias detection ───────────────────────────────────────────────────

/// Recursively walks `expr` and emits a Diagnostic for every unquoted
/// identifier that matches one of the SELECT-list aliases.
fn collect_lateral_alias_refs(
    expr: &Expr,
    aliases: &HashSet<String>,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Identifier(ident) => {
            check_ident(ident, aliases, rule, ctx, diags);
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_lateral_alias_refs(left, aliases, rule, ctx, diags);
            collect_lateral_alias_refs(right, aliases, rule, ctx, diags);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            collect_lateral_alias_refs(inner, aliases, rule, ctx, diags);
        }
        Expr::Nested(inner) => {
            collect_lateral_alias_refs(inner, aliases, rule, ctx, diags);
        }
        Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
            collect_lateral_alias_refs(inner, aliases, rule, ctx, diags);
        }
        Expr::Between {
            expr: e, low, high, ..
        } => {
            collect_lateral_alias_refs(e, aliases, rule, ctx, diags);
            collect_lateral_alias_refs(low, aliases, rule, ctx, diags);
            collect_lateral_alias_refs(high, aliases, rule, ctx, diags);
        }
        Expr::InList { expr: e, list, .. } => {
            collect_lateral_alias_refs(e, aliases, rule, ctx, diags);
            for item in list {
                collect_lateral_alias_refs(item, aliases, rule, ctx, diags);
            }
        }
        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                collect_lateral_alias_refs(op, aliases, rule, ctx, diags);
            }
            for cond in conditions {
                collect_lateral_alias_refs(cond, aliases, rule, ctx, diags);
            }
            for res in results {
                collect_lateral_alias_refs(res, aliases, rule, ctx, diags);
            }
            if let Some(else_e) = else_result {
                collect_lateral_alias_refs(else_e, aliases, rule, ctx, diags);
            }
        }
        Expr::Function(func) => {
            if let sqlparser::ast::FunctionArguments::List(list) = &func.args {
                for arg in &list.args {
                    let fae = match arg {
                        sqlparser::ast::FunctionArg::Named { arg, .. }
                        | sqlparser::ast::FunctionArg::ExprNamed { arg, .. }
                        | sqlparser::ast::FunctionArg::Unnamed(arg) => arg,
                    };
                    if let sqlparser::ast::FunctionArgExpr::Expr(e) = fae {
                        collect_lateral_alias_refs(e, aliases, rule, ctx, diags);
                    }
                }
            }
        }
        // Do not descend into subqueries here — they have their own scope.
        _ => {}
    }
}

fn check_ident(
    ident: &Ident,
    aliases: &HashSet<String>,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Only flag unquoted identifiers (quote_style is None).
    if ident.quote_style.is_some() {
        return;
    }

    let name_lower = ident.value.to_lowercase();
    if !aliases.contains(&name_lower) {
        return;
    }

    let offset = find_identifier_offset(&ctx.source, &ident.value);
    let (line, col) = offset_to_line_col(&ctx.source, offset);

    diags.push(Diagnostic {
        rule,
        message: format!(
            "Column alias '{}' used in WHERE/GROUP BY/HAVING — lateral column aliases are not supported by most databases",
            ident.value
        ),
        line,
        col,
    });
}

// ── Source-text helpers ───────────────────────────────────────────────────────

/// Finds the byte offset of the first whole-word, case-insensitive occurrence
/// of `name` in `source`, skipping positions inside strings/comments.
/// Returns 0 if not found.
fn find_identifier_offset(source: &str, name: &str) -> usize {
    let bytes = source.as_bytes();
    let skip_map = SkipMap::build(source);
    let name_bytes: Vec<u8> = name.bytes().map(|b| b.to_ascii_lowercase()).collect();
    let name_len = name_bytes.len();
    let src_len = bytes.len();

    let mut i = 0usize;

    while i + name_len <= src_len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }

        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        let matches = bytes[i..i + name_len]
            .iter()
            .zip(name_bytes.iter())
            .all(|(&a, &b)| a.to_ascii_lowercase() == b);

        if matches {
            let after = i + name_len;
            let after_ok = after >= src_len || !is_word_char(bytes[after]);
            let all_code = (i..i + name_len).all(|k| skip_map.is_code(k));

            if after_ok && all_code {
                return i;
            }
        }

        i += 1;
    }

    0
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
