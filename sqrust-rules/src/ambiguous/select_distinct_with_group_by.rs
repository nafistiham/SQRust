use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Distinct, GroupByExpr, Query, SetExpr, Statement, TableFactor};

pub struct SelectDistinctWithGroupBy;

impl Rule for SelectDistinctWithGroupBy {
    fn name(&self) -> &'static str {
        "Ambiguous/SelectDistinctWithGroupBy"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let mut occurrence = 0usize;
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &ctx.source, &mut occurrence, &mut diags);
            }
        }
        diags
    }
}

fn check_query(
    query: &Query,
    source: &str,
    occurrence: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // Recurse into CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, occurrence, diags);
        }
    }
    check_set_expr(&query.body, source, occurrence, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    source: &str,
    occurrence: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            // Only flag plain DISTINCT, not DISTINCT ON(...).
            let has_distinct = matches!(sel.distinct, Some(Distinct::Distinct));

            let has_group_by = match &sel.group_by {
                GroupByExpr::All(_) => true,
                GroupByExpr::Expressions(exprs, _) => !exprs.is_empty(),
            };

            if has_distinct && has_group_by {
                let (line, col) = find_nth_keyword_position(source, "DISTINCT", *occurrence);
                *occurrence += 1;
                diags.push(Diagnostic {
                    rule: "Ambiguous/SelectDistinctWithGroupBy",
                    message: "SELECT DISTINCT with GROUP BY is redundant; GROUP BY already deduplicates".to_string(),
                    line,
                    col,
                });
            }

            // Recurse into subqueries inside the FROM / JOIN clauses.
            for table in &sel.from {
                recurse_table_factor(&table.relation, source, occurrence, diags);
                for join in &table.joins {
                    recurse_table_factor(&join.relation, source, occurrence, diags);
                }
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, occurrence, diags);
            check_set_expr(right, source, occurrence, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, source, occurrence, diags);
        }
        _ => {}
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    source: &str,
    occurrence: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, occurrence, diags);
    }
}

/// Finds the `n`-th (0-indexed) occurrence of `keyword` (case-insensitive,
/// word-boundary-checked) in `source` and returns a 1-indexed (line, col).
/// Falls back to (1, 1) if not found.
fn find_nth_keyword_position(source: &str, keyword: &str, n: usize) -> (usize, usize) {
    let upper = source.to_uppercase();
    let kw_upper = keyword.to_uppercase();
    let bytes = upper.as_bytes();
    let kw_bytes = kw_upper.as_bytes();
    let kw_len = kw_bytes.len();

    let mut found = 0usize;
    let mut i = 0;
    while i + kw_len <= bytes.len() {
        if bytes[i..i + kw_len] == *kw_bytes {
            let before_ok =
                i == 0 || (!bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_');
            let after = i + kw_len;
            let after_ok = after >= bytes.len()
                || (!bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_');
            if before_ok && after_ok {
                if found == n {
                    return offset_to_line_col(source, i);
                }
                found += 1;
            }
        }
        i += 1;
    }
    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
