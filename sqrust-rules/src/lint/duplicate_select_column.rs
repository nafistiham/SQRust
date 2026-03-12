use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SelectItem, SetExpr, Statement, TableFactor};
use std::collections::HashMap;

pub struct DuplicateSelectColumn;

impl Rule for DuplicateSelectColumn {
    fn name(&self) -> &'static str {
        "Lint/DuplicateSelectColumn"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &ctx.source, &mut diags);
            }
        }
        diags
    }
}

fn check_query(query: &Query, source: &str, diags: &mut Vec<Diagnostic>) {
    // Recurse into CTEs first.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, diags);
        }
    }
    check_set_expr(&query.body, source, diags);
}

fn check_set_expr(expr: &SetExpr, source: &str, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            // Build a canonical name for each projection item.
            let mut seen: HashMap<String, usize> = HashMap::new();
            let mut dupes: Vec<String> = Vec::new();

            for item in &sel.projection {
                let canonical = match item {
                    SelectItem::UnnamedExpr(expr) => canonical_expr_name(expr),
                    SelectItem::ExprWithAlias { alias, .. } => {
                        Some(alias.value.to_lowercase())
                    }
                    // Wildcards and qualified wildcards are ignored.
                    SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => None,
                };

                if let Some(name) = canonical {
                    let count = seen.entry(name.clone()).or_insert(0);
                    *count += 1;
                    if *count == 2 {
                        dupes.push(name);
                    }
                }
            }

            for dupe in &dupes {
                let (line, col) = find_identifier_position(source, dupe);
                diags.push(Diagnostic {
                    rule: "Lint/DuplicateSelectColumn",
                    message: format!(
                        "Column '{}' is selected more than once in this SELECT",
                        dupe
                    ),
                    line,
                    col,
                });
            }

            // Recurse into subqueries in FROM clause.
            for table in &sel.from {
                check_table_factor(&table.relation, source, diags);
                for join in &table.joins {
                    check_table_factor(&join.relation, source, diags);
                }
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, diags);
            check_set_expr(right, source, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, source, diags);
        }
        _ => {}
    }
}

fn check_table_factor(tf: &TableFactor, source: &str, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, diags);
    }
}

/// Returns a canonical lowercase name for an expression used as a column reference.
/// Returns `None` for expressions that can't be meaningfully compared (literals, function calls, etc.).
fn canonical_expr_name(expr: &sqlparser::ast::Expr) -> Option<String> {
    match expr {
        sqlparser::ast::Expr::Identifier(id) => Some(id.value.to_lowercase()),
        sqlparser::ast::Expr::CompoundIdentifier(parts) => {
            // Use the last part (the column name), ignoring table/schema qualifiers.
            parts.last().map(|id| id.value.to_lowercase())
        }
        _ => None,
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
