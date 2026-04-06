use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, SelectItem, SetExpr, Statement, TableFactor};

pub struct SelfAlias;

impl Rule for SelfAlias {
    fn name(&self) -> &'static str {
        "Lint/SelfAlias"
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
                    if let Some(col_name) = extract_column_name(expr) {
                        if alias.value.to_lowercase() == col_name.to_lowercase() {
                            let (line, col) =
                                find_alias_position(source, &col_name, &alias.value);
                            diags.push(Diagnostic {
                                rule: "Lint/SelfAlias",
                                message: format!(
                                    "Column '{}' is aliased to itself; \
                                     remove the redundant AS {} clause",
                                    col_name, alias.value
                                ),
                                line,
                                col,
                            });
                        }
                    }
                }
            }

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

fn extract_column_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Identifier(ident) => Some(ident.value.clone()),
        Expr::CompoundIdentifier(parts) => parts.last().map(|id| id.value.clone()),
        _ => None,
    }
}

/// Find the position of `AS <alias>` in source after the column name appears.
/// Returns 1-indexed (line, col). Falls back to (1, 1).
fn find_alias_position(source: &str, col_name: &str, alias: &str) -> (usize, usize) {
    let source_lower = source.to_lowercase();
    let alias_lower = alias.to_lowercase();
    let col_lower = col_name.to_lowercase();
    let bytes = source_lower.as_bytes();
    let src_len = bytes.len();

    // Find the column name first, then look for AS <alias> after it.
    let mut search_from = 0usize;
    while search_from < src_len {
        let Some(col_rel) = source_lower[search_from..].find(col_lower.as_str()) else {
            break;
        };
        let col_abs = search_from + col_rel;

        let col_before_ok = col_abs == 0 || {
            let b = bytes[col_abs - 1];
            !b.is_ascii_alphanumeric() && b != b'_' && b != b'"'
        };
        let col_after = col_abs + col_lower.len();
        let col_after_ok = col_after >= src_len || {
            let b = bytes[col_after];
            !b.is_ascii_alphanumeric() && b != b'_' && b != b'"'
        };

        if col_before_ok && col_after_ok {
            // Now search for AS <alias> after this column position.
            let after_col = col_abs + col_lower.len();
            let as_pattern = format!("as {}", alias_lower);
            let remaining = &source_lower[after_col..];
            if let Some(as_rel) = remaining.find(as_pattern.as_str()) {
                let as_abs = after_col + as_rel;
                // Move past "AS " to point at the alias name.
                let alias_abs = as_abs + 3;
                if alias_abs < src_len {
                    return offset_to_line_col(source, alias_abs);
                }
            }
        }
        search_from = col_abs + 1;
    }

    // Fallback: find the alias as a whole word anywhere.
    let mut search_from = 0usize;
    while search_from < src_len {
        let Some(rel) = source_lower[search_from..].find(alias_lower.as_str()) else {
            break;
        };
        let abs = search_from + rel;

        let before_ok = abs == 0 || {
            let b = bytes[abs - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };
        let after = abs + alias_lower.len();
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

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
