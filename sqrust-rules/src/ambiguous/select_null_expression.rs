use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SelectItem, SetExpr, Statement, TableFactor, Value};

pub struct SelectNullExpression;

impl Rule for SelectNullExpression {
    fn name(&self) -> &'static str {
        "Ambiguous/SelectNullExpression"
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

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(query: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Visit CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, diags);
        }
    }

    check_set_expr(&query.body, ctx, diags);
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            // Check projection items for bare NULL (no alias).
            for item in &sel.projection {
                if let SelectItem::UnnamedExpr(sqlparser::ast::Expr::Value(Value::Null)) = item {
                    let (line, col) = find_null_pos(&ctx.source);
                    diags.push(Diagnostic {
                        rule: "Ambiguous/SelectNullExpression",
                        message: "Selecting a literal NULL without an alias; add an alias to clarify intent".to_string(),
                        line,
                        col,
                    });
                }
            }

            // Recurse into subqueries in FROM / JOIN clauses.
            for twj in &sel.from {
                check_table_factor(&twj.relation, ctx, diags);
                for join in &twj.joins {
                    check_table_factor(&join.relation, ctx, diags);
                }
            }
        }
        SetExpr::Query(inner) => check_query(inner, ctx, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, diags);
            check_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}

fn check_table_factor(tf: &TableFactor, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, ctx, diags);
    }
}

// ── position helper ───────────────────────────────────────────────────────────

/// Scan source for the first word-boundary `NULL` keyword (case-insensitive)
/// and return its 1-indexed (line, col). Falls back to (1, 1).
fn find_null_pos(source: &str) -> (usize, usize) {
    let keyword = "NULL";
    let upper_src = source.to_uppercase();
    let kw_len = keyword.len();
    let bytes = upper_src.as_bytes();
    let len = bytes.len();

    let mut pos = 0;
    while pos + kw_len <= len {
        if let Some(rel) = upper_src[pos..].find(keyword) {
            let abs = pos + rel;

            let before_ok = abs == 0 || {
                let b = bytes[abs - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            let after = abs + kw_len;
            let after_ok = after >= len || {
                let b = bytes[after];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

            if before_ok && after_ok {
                return line_col(source, abs);
            }

            pos = abs + 1;
        } else {
            break;
        }
    }

    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
