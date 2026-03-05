use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SelectItem, SetExpr, Statement, TableFactor};

pub struct SelectStarWithOtherColumns;

impl Rule for SelectStarWithOtherColumns {
    fn name(&self) -> &'static str {
        "Ambiguous/SelectStarWithOtherColumns"
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
            // A wildcard is `SelectItem::Wildcard` (bare `*`) or
            // `SelectItem::QualifiedWildcard` (e.g. `t.*`).
            // An "other" column is anything that is not a wildcard variant.
            let has_wildcard = sel.projection.iter().any(|item| {
                matches!(
                    item,
                    SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
                )
            });
            let has_other = sel.projection.iter().any(|item| {
                !matches!(
                    item,
                    SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
                )
            });

            if has_wildcard && has_other {
                // Find the SELECT keyword position to report a useful location.
                let (line, col) = find_select_position(source);
                diags.push(Diagnostic {
                    rule,
                    message: "Avoid mixing SELECT * with explicit columns; either use * alone or list all columns explicitly".to_string(),
                    line,
                    col,
                });
            }

            // Recurse into subqueries inside the FROM / JOIN clauses.
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

/// Finds the byte position of the first `SELECT` keyword in `source` and
/// returns a 1-indexed (line, col). Falls back to (1, 1) if not found.
fn find_select_position(source: &str) -> (usize, usize) {
    let upper = source.to_uppercase();
    let bytes = upper.as_bytes();
    let kw = b"SELECT";

    let mut i = 0;
    while i + kw.len() <= bytes.len() {
        if bytes[i..i + kw.len()].eq_ignore_ascii_case(kw) {
            // Check word boundaries.
            let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_';
            let after = i + kw.len();
            let after_ok = after >= bytes.len()
                || !bytes[after].is_ascii_alphanumeric() && bytes[after] != b'_';
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
