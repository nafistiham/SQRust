use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SetExpr, Statement, TableFactor, TableWithJoins};
use std::collections::HashMap;

pub struct TableAliasConflict;

impl Rule for TableAliasConflict {
    fn name(&self) -> &'static str {
        "Ambiguous/TableAliasConflict"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.name(), &mut diags);
            }
        }
        diags
    }
}

fn check_query(query: &Query, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    // Recurse into CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, diags);
        }
    }
    check_set_expr(&query.body, rule, diags);
}

fn check_set_expr(expr: &SetExpr, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            // Collect effective names for all table references in this SELECT's FROM clause.
            // Key: lowercased effective name. Value: first occurrence line/col.
            let mut seen: HashMap<String, (usize, usize)> = HashMap::new();
            let mut reported: std::collections::HashSet<String> = std::collections::HashSet::new();

            for twj in &sel.from {
                collect_from_item(twj, rule, &mut seen, &mut reported, diags);
            }

            // Recurse into subqueries in FROM / JOIN — but as separate scopes.
            for twj in &sel.from {
                recurse_subqueries_in_from(twj, rule, diags);
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, rule, diags);
            check_set_expr(right, rule, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, rule, diags);
        }
        _ => {}
    }
}

/// Collects all table reference effective names from a `TableWithJoins` and checks for conflicts.
/// Does NOT recurse into subqueries — they are handled separately as independent scopes.
fn collect_from_item(
    twj: &TableWithJoins,
    rule: &'static str,
    seen: &mut HashMap<String, (usize, usize)>,
    reported: &mut std::collections::HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    collect_table_factor_name(&twj.relation, rule, seen, reported, diags);
    for join in &twj.joins {
        collect_table_factor_name(&join.relation, rule, seen, reported, diags);
    }
}

/// Extracts the effective name from a `TableFactor` and checks for conflicts.
/// For `TableFactor::Table`: uses alias if present, else last part of table name.
/// For other variants (Derived, etc.): skips (handled as subquery scope).
fn collect_table_factor_name(
    tf: &TableFactor,
    rule: &'static str,
    seen: &mut HashMap<String, (usize, usize)>,
    reported: &mut std::collections::HashSet<String>,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Table { name, alias, .. } = tf {
        let effective = if let Some(table_alias) = alias {
            table_alias.name.value.to_lowercase()
        } else {
            // Use the last part of the qualified name as the effective name.
            name.0
                .last()
                .map(|ident| ident.value.to_lowercase())
                .unwrap_or_default()
        };

        if effective.is_empty() {
            return;
        }

        if seen.contains_key(&effective) {
            // Only report once per conflicting alias per SELECT scope.
            if reported.insert(effective.clone()) {
                diags.push(Diagnostic {
                    rule,
                    message: format!(
                        "Table alias '{}' is used more than once in this FROM clause",
                        effective
                    ),
                    line: 1,
                    col: 1,
                });
            }
        } else {
            seen.insert(effective, (1, 1));
        }
    }
}

/// Recurses into subqueries that appear as `Derived` table factors (independent scopes).
fn recurse_subqueries_in_from(twj: &TableWithJoins, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    recurse_table_factor_subquery(&twj.relation, rule, diags);
    for join in &twj.joins {
        recurse_table_factor_subquery(&join.relation, rule, diags);
    }
}

fn recurse_table_factor_subquery(
    tf: &TableFactor,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, rule, diags);
    }
}
