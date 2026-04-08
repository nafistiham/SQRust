use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SelectItem, SetExpr, SetOperator, Statement};

pub struct UnionColumnAlias;

impl Rule for UnionColumnAlias {
    fn name(&self) -> &'static str {
        "Structure/UnionColumnAlias"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &mut diags);
            }
        }

        diags
    }
}

fn check_query(query: &Query, diags: &mut Vec<Diagnostic>) {
    // Check CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, diags);
        }
    }

    check_set_expr(&query.body, diags);
}

fn check_set_expr(expr: &SetExpr, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(_) => {}
        SetExpr::Query(inner) => {
            check_query(inner, diags);
        }
        SetExpr::SetOperation {
            op,
            left,
            right,
            ..
        } => {
            if *op == SetOperator::Union {
                // `right` is always a non-first branch in sqlparser-rs UNION chains.
                collect_aliases_from_non_first_branch(right, diags);
                // Recurse into `left` — may contain more UNION operations.
                check_set_expr(left, diags);
            } else {
                // INTERSECT / EXCEPT — just recurse without flagging.
                check_set_expr(left, diags);
                check_set_expr(right, diags);
            }
        }
        _ => {}
    }
}

/// Collect all aliases from a non-first UNION branch's SELECT projection.
fn collect_aliases_from_non_first_branch(expr: &SetExpr, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            for item in &sel.projection {
                if let SelectItem::ExprWithAlias { .. } = item {
                    diags.push(Diagnostic {
                        rule: "Structure/UnionColumnAlias",
                        message: "Column alias in non-first UNION branch is ignored — aliases only apply to the first SELECT"
                            .to_string(),
                        line: 1,
                        col: 1,
                    });
                }
            }
        }
        SetExpr::Query(inner) => {
            // Wrapped query — check its body.
            collect_aliases_from_non_first_branch(&inner.body, diags);
        }
        // Nested set operations in non-first branch — recurse.
        SetExpr::SetOperation { op, left, right, .. } => {
            if *op == SetOperator::Union {
                collect_aliases_from_non_first_branch(right, diags);
                collect_aliases_from_non_first_branch(left, diags);
            } else {
                collect_aliases_from_non_first_branch(left, diags);
                collect_aliases_from_non_first_branch(right, diags);
            }
        }
        _ => {}
    }
}
