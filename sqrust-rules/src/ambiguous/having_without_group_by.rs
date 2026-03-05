use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{GroupByExpr, Query, SetExpr, Statement, TableFactor};

pub struct HavingWithoutGroupBy;

impl Rule for HavingWithoutGroupBy {
    fn name(&self) -> &'static str {
        "Ambiguous/HavingWithoutGroupBy"
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
            let has_having = sel.having.is_some();

            // `GroupByExpr::Expressions(exprs, _)` with empty `exprs` means no GROUP BY.
            // `GroupByExpr::All(_)` means `GROUP BY ALL` — that counts as having a GROUP BY.
            let has_group_by = match &sel.group_by {
                GroupByExpr::All(_) => true,
                GroupByExpr::Expressions(exprs, _) => !exprs.is_empty(),
            };

            if has_having && !has_group_by {
                // Find HAVING keyword position in the source text.
                let (line, col) = find_keyword_position(source, "HAVING");
                diags.push(Diagnostic {
                    rule,
                    message: "HAVING without GROUP BY; did you mean WHERE?".to_string(),
                    line,
                    col,
                });
            }

            // Recurse into subqueries inside FROM / JOIN clauses.
            for table in &sel.from {
                recurse_table_factor(&table.relation, source, rule, diags);
                for join in &table.joins {
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

/// Finds the first occurrence of `keyword` (case-insensitive, word-boundary-checked)
/// in `source` and returns a 1-indexed (line, col). Falls back to (1, 1) if not found.
fn find_keyword_position(source: &str, keyword: &str) -> (usize, usize) {
    let upper = source.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let bytes = upper.as_bytes();
    let kw_bytes = kw_upper.as_bytes();
    let kw_len = kw_bytes.len();

    let mut i = 0;
    while i + kw_len <= bytes.len() {
        if bytes[i..i + kw_len] == *kw_bytes {
            let before_ok = i == 0
                || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let after = i + kw_len;
            let after_ok = after >= bytes.len()
                || (!bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_');
            if before_ok && after_ok {
                return offset_to_line_col(source, i);
            }
        }
        i += 1;
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
