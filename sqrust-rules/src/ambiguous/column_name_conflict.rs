use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, SelectItem, SetExpr, Statement};
use std::collections::{HashMap, HashSet};

pub struct ColumnNameConflict;

impl Rule for ColumnNameConflict {
    fn name(&self) -> &'static str {
        "Ambiguous/ColumnNameConflict"
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
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, diags);
        }
    }
    check_set_expr(&query.body, rule, diags);
}

fn check_set_expr(body: &SetExpr, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match body {
        SetExpr::Select(sel) => {
            // Check this SELECT's projection for duplicate output names.
            check_projection(&sel.projection, rule, diags);

            // Recurse into any subqueries in the FROM / JOIN clauses.
            for table in &sel.from {
                recurse_table_subqueries(&table.relation, rule, diags);
                for join in &table.joins {
                    recurse_table_subqueries(&join.relation, rule, diags);
                }
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

fn recurse_table_subqueries(
    tf: &sqlparser::ast::TableFactor,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let sqlparser::ast::TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, rule, diags);
    }
}

/// Extract the output name for a SelectItem, if determinable.
///
/// Rules:
/// - `ExprWithAlias { alias, .. }` → alias value (lowercased)
/// - `UnnamedExpr(Identifier(ident))` → ident value (lowercased)
/// - `UnnamedExpr(CompoundIdentifier(parts))` → last part value (lowercased)
/// - `Wildcard` / `QualifiedWildcard` → None (skipped)
/// - Other `UnnamedExpr` → None (no predictable name)
fn output_name(item: &SelectItem) -> Option<String> {
    match item {
        SelectItem::ExprWithAlias { alias, .. } => Some(alias.value.to_lowercase()),
        SelectItem::UnnamedExpr(expr) => match expr {
            Expr::Identifier(ident) => Some(ident.value.to_lowercase()),
            Expr::CompoundIdentifier(parts) => {
                parts.last().map(|p| p.value.to_lowercase())
            }
            _ => None,
        },
        SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => None,
    }
}

fn check_projection(
    projection: &[SelectItem],
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    // Map: lowercase name → first occurrence index (for ordering determinism).
    let mut seen: HashMap<String, usize> = HashMap::new();
    // Track names already reported to avoid duplicate diagnostics.
    let mut reported: HashSet<String> = HashSet::new();

    for (idx, item) in projection.iter().enumerate() {
        if let Some(name) = output_name(item) {
            if seen.contains_key(&name) {
                if reported.insert(name.clone()) {
                    diags.push(Diagnostic {
                        rule,
                        message: format!(
                            "Column name '{}' appears more than once in SELECT list",
                            name
                        ),
                        line: 1,
                        col: 1,
                    });
                }
            } else {
                seen.insert(name, idx);
            }
        }
    }
}
