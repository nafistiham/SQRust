use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, Select, SelectItem, SetExpr, Statement};

pub struct UnionColumnMismatch;

impl Rule for UnionColumnMismatch {
    fn name(&self) -> &'static str {
        "Ambiguous/UnionColumnMismatch"
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
        SetExpr::SetOperation { op: _, left, right, .. } => {
            // Collect non-wildcard column counts from all leaf branches.
            let counts = collect_select_column_counts(body);

            if counts.len() >= 2 {
                // The first count is the "expected" baseline.
                let expected = counts[0];
                for &count in &counts[1..] {
                    if count != expected {
                        diags.push(Diagnostic {
                            rule,
                            message: format!(
                                "UNION branch has {} columns but expected {}",
                                count, expected
                            ),
                            line: 1,
                            col: 1,
                        });
                        // Only report once per set operation.
                        break;
                    }
                }
            }

            // Also recurse into subqueries within the branches' FROM clauses.
            recurse_subqueries_in_set_expr(left, rule, diags);
            recurse_subqueries_in_set_expr(right, rule, diags);
        }
        SetExpr::Select(sel) => {
            // Recurse into subqueries inside this SELECT's FROM clause.
            for table in &sel.from {
                recurse_table_subqueries(&table.relation, rule, diags);
                for join in &table.joins {
                    recurse_table_subqueries(&join.relation, rule, diags);
                }
            }
        }
        SetExpr::Query(inner) => {
            check_query(inner, rule, diags);
        }
        _ => {}
    }
}

/// Recurse into the FROM-clause subqueries of a SetExpr, but do NOT re-check the set
/// operation itself (that is handled by check_set_expr at the top-level call).
fn recurse_subqueries_in_set_expr(
    body: &SetExpr,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    match body {
        SetExpr::Select(sel) => {
            for table in &sel.from {
                recurse_table_subqueries(&table.relation, rule, diags);
                for join in &table.joins {
                    recurse_table_subqueries(&join.relation, rule, diags);
                }
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            // Nested set operations within a branch are handled at check_set_expr level.
            recurse_subqueries_in_set_expr(left, rule, diags);
            recurse_subqueries_in_set_expr(right, rule, diags);
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

/// Recursively collect column counts for all non-wildcard leaf SELECT branches.
/// Returns an empty Vec if every branch contains a wildcard (no information).
fn collect_select_column_counts(body: &SetExpr) -> Vec<usize> {
    match body {
        SetExpr::Select(sel) => {
            if has_wildcard(sel) {
                // Wildcard branch — skip, count unknown.
                Vec::new()
            } else {
                vec![sel.projection.len()]
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            let mut counts = collect_select_column_counts(left);
            counts.extend(collect_select_column_counts(right));
            counts
        }
        SetExpr::Query(inner) => collect_select_column_counts(&inner.body),
        _ => Vec::new(),
    }
}

fn has_wildcard(sel: &Select) -> bool {
    sel.projection.iter().any(|item| {
        matches!(
            item,
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
        )
    })
}
