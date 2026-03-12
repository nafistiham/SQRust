use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};
use std::collections::HashSet;

pub struct KeywordIdentifier;

/// SQL keywords that are commonly (mis)used as unquoted column or alias names.
/// These are problematic because they conflict with SQL reserved words, causing
/// portability issues and readability problems across different SQL dialects.
static BLOCKED_KEYWORDS: &[&str] = &[
    "date",
    "time",
    "timestamp",
    "year",
    "month",
    "day",
    "hour",
    "minute",
    "second",
    "name",
    "value",
    "status",
    "type",
    "comment",
    "user",
    "key",
    "data",
    "text",
    "level",
    "position",
    "primary",
    "references",
    "check",
    "default",
    "constraint",
];

impl Rule for KeywordIdentifier {
    fn name(&self) -> &'static str {
        "Lint/KeywordIdentifier"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let blocked: HashSet<&str> = BLOCKED_KEYWORDS.iter().copied().collect();
        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &ctx.source, &blocked, &mut diags);
            }
        }
        diags
    }
}

fn check_query(
    query: &Query,
    source: &str,
    blocked: &HashSet<&str>,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, blocked, diags);
        }
    }
    check_set_expr(&query.body, source, blocked, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    source: &str,
    blocked: &HashSet<&str>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, source, blocked, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, blocked, diags);
            check_set_expr(right, source, blocked, diags);
        }
        SetExpr::Query(inner) => check_query(inner, source, blocked, diags),
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    source: &str,
    blocked: &HashSet<&str>,
    diags: &mut Vec<Diagnostic>,
) {
    // Check SELECT projection items.
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(expr) => {
                check_expr_for_keyword(expr, source, blocked, diags);
            }
            SelectItem::ExprWithAlias { expr, alias } => {
                // Check the expression itself.
                check_expr_for_keyword(expr, source, blocked, diags);
                // Check the alias.
                if alias.quote_style.is_none() {
                    let lower = alias.value.to_lowercase();
                    if blocked.contains(lower.as_str()) {
                        let (line, col) = find_identifier_position(source, &lower);
                        diags.push(Diagnostic {
                            rule: "Lint/KeywordIdentifier",
                            message: format!(
                                "'{}' is a SQL keyword used as an identifier — consider renaming or quoting it",
                                lower
                            ),
                            line,
                            col,
                        });
                    }
                }
            }
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => {}
        }
    }

    // Recurse into subqueries in FROM.
    for table in &sel.from {
        check_table_factor(&table.relation, source, blocked, diags);
        for join in &table.joins {
            check_table_factor(&join.relation, source, blocked, diags);
        }
    }
}

fn check_table_factor(
    tf: &TableFactor,
    source: &str,
    blocked: &HashSet<&str>,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, blocked, diags);
    }
}

/// Checks an expression for unquoted identifiers that match blocked keywords.
/// Only checks top-level identifiers (column references), not function names or
/// expressions used as types.
fn check_expr_for_keyword(
    expr: &Expr,
    source: &str,
    blocked: &HashSet<&str>,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Identifier(ident) => {
            if ident.quote_style.is_none() {
                let lower = ident.value.to_lowercase();
                if blocked.contains(lower.as_str()) {
                    let (line, col) = find_identifier_position(source, &lower);
                    diags.push(Diagnostic {
                        rule: "Lint/KeywordIdentifier",
                        message: format!(
                            "'{}' is a SQL keyword used as an identifier — consider renaming or quoting it",
                            lower
                        ),
                        line,
                        col,
                    });
                }
            }
        }
        Expr::CompoundIdentifier(parts) => {
            // Check the last part — the column name portion (e.g. `t.name` → check `name`).
            if let Some(last) = parts.last() {
                if last.quote_style.is_none() {
                    let lower = last.value.to_lowercase();
                    if blocked.contains(lower.as_str()) {
                        let (line, col) = find_identifier_position(source, &lower);
                        diags.push(Diagnostic {
                            rule: "Lint/KeywordIdentifier",
                            message: format!(
                                "'{}' is a SQL keyword used as an identifier — consider renaming or quoting it",
                                lower
                            ),
                            line,
                            col,
                        });
                    }
                }
            }
        }
        // Do not recurse into function calls or other complex expressions —
        // they are not column references and would produce false positives.
        _ => {}
    }
}

/// Finds the first whole-word occurrence of `name` in `source` (case-insensitive)
/// and returns its 1-indexed (line, col). Falls back to (1, 1) if not found.
fn find_identifier_position(source: &str, name: &str) -> (usize, usize) {
    let source_lower = source.to_lowercase();
    let bytes = source_lower.as_bytes();
    let name_len = name.len();
    let src_len = bytes.len();

    let mut search_from = 0usize;
    while search_from < src_len {
        let Some(rel) = source_lower[search_from..].find(name) else {
            break;
        };
        let abs = search_from + rel;

        let before_ok = abs == 0 || {
            let b = bytes[abs - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };
        let after = abs + name_len;
        let after_ok = after >= src_len || {
            let b = bytes[after];
            !b.is_ascii_alphanumeric() && b != b'_'
        };

        if before_ok && after_ok {
            return offset_to_line_col(source, abs);
        }
        search_from = abs + 1;
    }

    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
