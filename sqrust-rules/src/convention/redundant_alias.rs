use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, SelectItem, SetExpr, Statement, TableFactor};

pub struct RedundantAlias;

impl Rule for RedundantAlias {
    fn name(&self) -> &'static str {
        "Convention/RedundantAlias"
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
            for item in &sel.projection {
                if let SelectItem::ExprWithAlias { expr, alias } = item {
                    let alias_lower = alias.value.to_lowercase();
                    let col_name = extract_column_name(expr);
                    if let Some(name) = col_name {
                        if name.to_lowercase() == alias_lower {
                            let (line, col) = find_alias_position(source, &alias_lower);
                            diags.push(Diagnostic {
                                rule: "Convention/RedundantAlias",
                                message: format!(
                                    "Column alias '{}' is identical to the column name — alias is redundant",
                                    alias.value
                                ),
                                line,
                                col,
                            });
                        }
                    }
                }
            }

            // Recurse into subqueries in the FROM clause.
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

/// Extracts the bare column name from an expression.
/// For `Expr::Identifier(ident)` returns `ident.value`.
/// For `Expr::CompoundIdentifier(parts)` returns the last part's value.
/// Returns `None` for all other expressions.
fn extract_column_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Identifier(ident) => Some(&ident.value),
        Expr::CompoundIdentifier(parts) => parts.last().map(|p| p.value.as_str()),
        _ => None,
    }
}

/// Finds the first occurrence of `AS <alias>` (case-insensitive) in `source`
/// and returns its 1-indexed (line, col). Falls back to (1, 1) if not found.
fn find_alias_position(source: &str, alias: &str) -> (usize, usize) {
    let source_lower = source.to_lowercase();
    let pattern = format!("as {}", alias);

    let mut search_from = 0usize;
    while search_from < source_lower.len() {
        let Some(rel) = source_lower[search_from..].find(&pattern) else {
            break;
        };
        let abs = search_from + rel;
        let bytes = source_lower.as_bytes();

        let before_ok = abs == 0
            || {
                let b = bytes[abs - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

        let after_pos = abs + pattern.len();
        let after_ok = after_pos >= source_lower.len()
            || {
                let b = bytes[after_pos];
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
    let mut line = 1usize;
    let mut col = 1usize;
    for (i, ch) in source.char_indices() {
        if i == offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
