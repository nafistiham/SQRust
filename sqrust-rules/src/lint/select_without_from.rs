use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, SelectItem, SetExpr, Statement};

pub struct SelectWithoutFrom;

impl Rule for SelectWithoutFrom {
    fn name(&self) -> &'static str {
        "Lint/SelectWithoutFrom"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_set_expr(&query.body, self.name(), &mut diags);
            }
        }

        diags
    }
}

fn check_set_expr(expr: &SetExpr, rule_name: &'static str, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            if sel.from.is_empty() && !sel.projection.is_empty() {
                let has_non_literal = sel.projection.iter().any(|item| match item {
                    SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                        is_non_literal(e)
                    }
                    // Wildcard (*) without FROM is suspicious
                    SelectItem::Wildcard(_) => true,
                    _ => false,
                });

                if has_non_literal {
                    diags.push(Diagnostic {
                        rule: rule_name,
                        message:
                            "SELECT references columns or functions without a FROM clause"
                                .to_string(),
                        line: 1,
                        col: 1,
                    });
                }
            }
        }
        SetExpr::Query(inner) => {
            check_set_expr(&inner.body, rule_name, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, rule_name, diags);
            check_set_expr(right, rule_name, diags);
        }
        _ => {}
    }
}

/// Returns `true` if the expression is NOT a plain literal value (number, string,
/// boolean, NULL). Column references, function calls, and compound identifiers
/// all return `true`.
fn is_non_literal(expr: &Expr) -> bool {
    match expr {
        // Plain literal values (numbers, strings, booleans, NULL, TRUE, FALSE)
        Expr::Value(_) => false,
        // Everything else is non-literal
        _ => true,
    }
}
