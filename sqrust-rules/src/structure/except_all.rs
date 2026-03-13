use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SetExpr, SetOperator, SetQuantifier, Statement};

pub struct ExceptAll;

impl Rule for ExceptAll {
    fn name(&self) -> &'static str {
        "Structure/ExceptAll"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.name(), ctx, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, rule, ctx, diags);
        }
    }

    check_set_expr(&query.body, rule, ctx, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::SetOperation {
            op,
            set_quantifier,
            left,
            right,
        } => {
            // Check EXCEPT ALL or INTERSECT ALL
            let is_except_or_intersect = matches!(op, SetOperator::Except | SetOperator::Intersect);
            let is_all = matches!(set_quantifier, SetQuantifier::All);

            if is_except_or_intersect && is_all {
                let message = match op {
                    SetOperator::Except => {
                        "EXCEPT ALL is not supported by all databases — use EXCEPT (without ALL) for portability".to_string()
                    }
                    SetOperator::Intersect => {
                        "INTERSECT ALL is not supported by all databases — use INTERSECT (without ALL) for portability".to_string()
                    }
                    _ => unreachable!(),
                };

                diags.push(Diagnostic {
                    rule,
                    message,
                    line: 1,
                    col: 1,
                });
            }

            // Recurse into both branches to catch chained operations
            check_set_expr(left, rule, ctx, diags);
            check_set_expr(right, rule, ctx, diags);
        }
        SetExpr::Select(sel) => {
            // Recurse into subqueries inside FROM clause
            for twj in &sel.from {
                recurse_table_factor(&twj.relation, rule, ctx, diags);
                for join in &twj.joins {
                    recurse_table_factor(&join.relation, rule, ctx, diags);
                }
            }

            // Recurse into subqueries in WHERE, HAVING
            if let Some(selection) = &sel.selection {
                recurse_expr(selection, rule, ctx, diags);
            }
            if let Some(having) = &sel.having {
                recurse_expr(having, rule, ctx, diags);
            }
        }
        SetExpr::Query(inner) => {
            check_query(inner, rule, ctx, diags);
        }
        _ => {}
    }
}

fn recurse_table_factor(
    tf: &sqlparser::ast::TableFactor,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    if let sqlparser::ast::TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, rule, ctx, diags);
    }
}

fn recurse_expr(
    expr: &sqlparser::ast::Expr,
    rule: &'static str,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        sqlparser::ast::Expr::Subquery(q) => check_query(q, rule, ctx, diags),
        sqlparser::ast::Expr::InSubquery { subquery, .. } => {
            check_query(subquery, rule, ctx, diags)
        }
        sqlparser::ast::Expr::Exists { subquery, .. } => check_query(subquery, rule, ctx, diags),
        sqlparser::ast::Expr::BinaryOp { left, right, .. } => {
            recurse_expr(left, rule, ctx, diags);
            recurse_expr(right, rule, ctx, diags);
        }
        sqlparser::ast::Expr::Nested(inner) => recurse_expr(inner, rule, ctx, diags),
        _ => {}
    }
}
