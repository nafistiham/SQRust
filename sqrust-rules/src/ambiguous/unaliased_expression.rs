use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, SelectItem, SetExpr, Statement, TableFactor};

pub struct UnaliasedExpression;

impl Rule for UnaliasedExpression {
    fn name(&self) -> &'static str {
        "Ambiguous/UnaliasedExpression"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &ctx.source, self.name(), &mut diags);
            }
        }
        diags
    }
}

fn check_query(query: &Query, source: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    // Recurse into CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, rule, diags);
        }
    }
    check_set_expr(&query.body, source, rule, diags);
}

fn check_set_expr(expr: &SetExpr, source: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            for item in &sel.projection {
                if let SelectItem::UnnamedExpr(expr) = item {
                    if !is_simple_column_ref(expr) {
                        diags.push(Diagnostic {
                            rule,
                            message: "Expression in SELECT should have an explicit alias"
                                .to_string(),
                            line: 1,
                            col: 1,
                        });
                    }
                }
            }

            // Recurse into subqueries inside FROM / JOIN clauses.
            for twj in &sel.from {
                recurse_table_factor(&twj.relation, source, rule, diags);
                for join in &twj.joins {
                    recurse_table_factor(&join.relation, source, rule, diags);
                }
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, rule, diags);
            check_set_expr(right, source, rule, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, source, rule, diags);
        }
        _ => {}
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    source: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, rule, diags);
    }
}

/// Returns `true` for bare column references (`col` or `t.col`) that do not
/// need an alias to have a well-defined output column name.
fn is_simple_column_ref(expr: &Expr) -> bool {
    matches!(expr, Expr::Identifier(_) | Expr::CompoundIdentifier(_))
}
