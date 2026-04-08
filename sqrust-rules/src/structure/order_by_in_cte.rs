use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SetExpr, Statement};

pub struct OrderByInCte;

impl Rule for OrderByInCte {
    fn name(&self) -> &'static str {
        "Structure/OrderByInCte"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, ctx, &mut diags);
            }
        }

        diags
    }
}

fn check_query(query: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Check CTEs at this level.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            let cte_name = cte.alias.name.value.clone();
            check_cte_query(&cte.query, &cte_name, ctx, diags);
        }
    }

    // Recurse into the body to catch nested queries / subqueries.
    check_set_expr(&query.body, ctx, diags);
}

/// Check a CTE's inner query. Flag any ORDER BY, then recurse for nested CTEs.
fn check_cte_query(query: &Query, cte_name: &str, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Flag if ORDER BY is non-empty.
    if let Some(order_by) = &query.order_by {
        if !order_by.exprs.is_empty() {
            diags.push(Diagnostic {
                rule: "Structure/OrderByInCte",
                message: format!(
                    "ORDER BY inside CTE '{}' has no guaranteed effect — remove it or move to the outer query",
                    cte_name
                ),
                line: 1,
                col: 1,
            });
        }
    }

    // Recurse into nested CTEs inside this CTE body.
    if let Some(with) = &query.with {
        for nested_cte in &with.cte_tables {
            let nested_name = nested_cte.alias.name.value.clone();
            check_cte_query(&nested_cte.query, &nested_name, ctx, diags);
        }
    }

    // Recurse into body for any further nested queries.
    check_set_expr(&query.body, ctx, diags);
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(_) => {
            // Plain SELECT — no nested queries to recurse into at this level.
        }
        SetExpr::Query(inner) => {
            check_query(inner, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, diags);
            check_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}
