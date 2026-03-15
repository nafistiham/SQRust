use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SetExpr, Statement, TableFactor};
use std::collections::HashMap;

pub struct DuplicateCteNames;

impl Rule for DuplicateCteNames {
    fn name(&self) -> &'static str {
        "Lint/DuplicateCteNames"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, &ctx.source, &mut diags);
            }
        }
        diags
    }
}

/// Recursively walks a Query, checking its WITH clause for duplicate CTE names,
/// and then recurses into CTE bodies and the main query body.
fn check_query(query: &Query, source: &str, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &query.with {
        // Collect CTE names (lowercased) in order so we can detect duplicates
        // while preserving insertion order for deterministic output.
        let mut seen: HashMap<String, usize> = HashMap::new();
        // Track which names were reported as duplicates to avoid emitting twice.
        let mut reported: std::collections::HashSet<String> = std::collections::HashSet::new();

        for cte in &with.cte_tables {
            let name_lower = cte.alias.name.value.to_lowercase();
            let count = seen.entry(name_lower.clone()).or_insert(0);
            *count += 1;

            if *count == 2 && !reported.contains(&name_lower) {
                // Emit a single diagnostic for this duplicate name.
                reported.insert(name_lower.clone());
                let (line, col) = find_name_position(source, &cte.alias.name.value);
                diags.push(Diagnostic {
                    rule: "Lint/DuplicateCteNames",
                    message: format!(
                        "CTE name '{}' is used more than once in this WITH clause",
                        name_lower
                    ),
                    line,
                    col,
                });
            }

            // Recurse into the CTE's own body (may contain nested WITH).
            check_query(&cte.query, source, diags);
        }
    }

    // Recurse into the main query body.
    check_set_expr(&query.body, source, diags);
}

fn check_set_expr(expr: &SetExpr, source: &str, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            // Recurse into subqueries in FROM / JOIN.
            for table in &sel.from {
                check_table_factor(&table.relation, source, diags);
                for join in &table.joins {
                    check_table_factor(&join.relation, source, diags);
                }
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, diags);
            check_set_expr(right, source, diags);
        }
        SetExpr::Query(inner) => {
            check_query(inner, source, diags);
        }
        _ => {}
    }
}

fn check_table_factor(tf: &TableFactor, source: &str, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, diags);
    }
}

/// Finds the first occurrence of `name` as a whole word in `source` using ASCII
/// case-insensitive comparison. Operates on original bytes to keep offsets valid
/// even when `source` contains non-ASCII characters.
fn find_name_position(source: &str, name: &str) -> (usize, usize) {
    let src = source.as_bytes();
    let wrd: Vec<u8> = name.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let wlen = wrd.len();
    if wlen == 0 {
        return (1, 1);
    }
    let mut pos = 0usize;
    while pos + wlen <= src.len() {
        if src[pos..pos + wlen]
            .iter()
            .zip(wrd.iter())
            .all(|(a, b)| a.to_ascii_uppercase() == *b)
        {
            let before_ok = pos == 0 || {
                let b = src[pos - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            let after = pos + wlen;
            let after_ok = after >= src.len() || {
                let b = src[after];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
            if before_ok && after_ok {
                return offset_to_line_col(source, pos);
            }
        }
        pos += 1;
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
