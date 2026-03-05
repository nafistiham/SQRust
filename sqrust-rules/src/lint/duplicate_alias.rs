use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SelectItem, SetExpr, Statement, TableFactor};
use std::collections::HashMap;

pub struct DuplicateAlias;

impl Rule for DuplicateAlias {
    fn name(&self) -> &'static str {
        "Lint/DuplicateAlias"
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
    // Check the optional WITH clause (CTEs)
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
            // Collect aliases and find duplicates — report each duplicate alias once.
            let mut seen: HashMap<String, usize> = HashMap::new();
            // Preserve insertion order for deterministic output — collect dupes in order.
            let mut dupes: Vec<String> = Vec::new();

            for item in &sel.projection {
                if let SelectItem::ExprWithAlias { alias, .. } = item {
                    let name = alias.value.to_lowercase();
                    let count = seen.entry(name.clone()).or_insert(0);
                    *count += 1;
                    if *count == 2 {
                        // Record duplicate the first time the count hits 2.
                        dupes.push(name);
                    }
                }
            }

            for dupe in &dupes {
                // Try to locate `AS <alias>` in source for a useful position.
                let (line, col) = find_alias_position(source, dupe);
                diags.push(Diagnostic {
                    rule: "Lint/DuplicateAlias",
                    message: format!(
                        "Column alias '{}' is used more than once in this SELECT",
                        dupe
                    ),
                    line,
                    col,
                });
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
        // Query expressions (subqueries as SetExpr::Query) — recurse.
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

        // Check word boundary before 'as'.
        let before_ok = abs == 0
            || {
                let b = bytes[abs - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

        let after_pos = abs + pattern.len();
        // Check word boundary after alias.
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
