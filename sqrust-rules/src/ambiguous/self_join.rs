use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{JoinConstraint, JoinOperator, Query, SetExpr, Statement, TableFactor};
use std::collections::HashMap;

pub struct SelfJoin;

impl Rule for SelfJoin {
    fn name(&self) -> &'static str {
        "Ambiguous/SelfJoin"
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

fn check_query(query: &Query, source: &str, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, diags);
        }
    }
    check_set_expr(&query.body, source, diags);
}

fn check_set_expr(expr: &SetExpr, source: &str, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            for twj in &sel.from {
                // Collect all table references in this FROM + JOIN chain.
                let mut refs: Vec<(String, Option<String>)> = Vec::new();

                collect_table_ref(&twj.relation, &mut refs, source, diags);
                for join in &twj.joins {
                    collect_table_ref(&join.relation, &mut refs, source, diags);

                    // Also collect ON-clause subqueries.
                    if let JoinOperator::Inner(JoinConstraint::On(_))
                    | JoinOperator::LeftOuter(JoinConstraint::On(_))
                    | JoinOperator::RightOuter(JoinConstraint::On(_))
                    | JoinOperator::FullOuter(JoinConstraint::On(_)) = &join.join_operator
                    {
                        // ON expressions don't contain subqueries we need to recurse into here.
                    }
                }

                // Detect self-joins: same table name used twice without distinct aliases.
                detect_self_joins(&refs, source, diags);
            }

            // Recurse into subqueries in FROM (Derived table factors).
            for twj in &sel.from {
                recurse_subqueries_in_factor(&twj.relation, source, diags);
                for join in &twj.joins {
                    recurse_subqueries_in_factor(&join.relation, source, diags);
                }
            }
        }
        SetExpr::Query(inner) => check_query(inner, source, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, diags);
            check_set_expr(right, source, diags);
        }
        _ => {}
    }
}

/// Appends a (table_name_lowercase, alias_name_or_none) entry for each
/// `TableFactor::Table` found in `factor`. Derived factors (subqueries) are
/// NOT included here — they are handled separately via `recurse_subqueries_in_factor`.
fn collect_table_ref(
    factor: &TableFactor,
    refs: &mut Vec<(String, Option<String>)>,
    _source: &str,
    _diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Table { name, alias, .. } = factor {
        let table_name = name
            .0
            .last()
            .map(|i| i.value.to_lowercase())
            .unwrap_or_default();
        let alias_name = alias.as_ref().map(|a| a.name.value.to_lowercase());
        refs.push((table_name, alias_name));
    }
}

/// Recurse into derived (subquery) table factors.
fn recurse_subqueries_in_factor(
    factor: &TableFactor,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        check_query(subquery, source, diags);
    }
}

/// Given a list of (table_name, alias_option) pairs from a single FROM clause,
/// find cases where the same table appears twice and at least one occurrence
/// has no alias, or both have the same alias.
fn detect_self_joins(
    refs: &[(String, Option<String>)],
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    // Group by table name.
    // For each table, collect all aliases (None = no alias).
    let mut by_name: HashMap<&str, Vec<Option<&str>>> = HashMap::new();
    for (name, alias) in refs {
        by_name
            .entry(name.as_str())
            .or_default()
            .push(alias.as_deref());
    }

    for (table_name, aliases) in &by_name {
        if aliases.len() < 2 {
            continue;
        }

        // Self-join is ambiguous when at least one occurrence has no alias
        // OR when any two occurrences share the same alias.
        let is_ambiguous = aliases.iter().any(|a| a.is_none())
            || {
                // Check for duplicate aliases.
                let named: Vec<&str> = aliases.iter().filter_map(|a| *a).collect();
                has_duplicate(&named)
            };

        if is_ambiguous {
            // Find the second occurrence of the table name in the source text.
            let pos = find_second_occurrence(source, table_name);
            let (line, col) = offset_to_line_col(source, pos);
            diags.push(Diagnostic {
                rule: "Ambiguous/SelfJoin",
                message: format!(
                    "Table '{}' is joined to itself without distinct aliases",
                    table_name
                ),
                line,
                col,
            });
        }
    }
}

/// Returns true if any value in `names` appears more than once.
fn has_duplicate(names: &[&str]) -> bool {
    for i in 0..names.len() {
        for j in (i + 1)..names.len() {
            if names[i] == names[j] {
                return true;
            }
        }
    }
    false
}

/// Finds the byte offset of the second whole-word, case-insensitive occurrence
/// of `name` in `source`. Falls back to 0 (will resolve to line 1, col 1).
fn find_second_occurrence(source: &str, name: &str) -> usize {
    find_nth_occurrence(source, name, 1)
}

/// Finds the byte offset of the `nth` (0-indexed) whole-word, case-insensitive
/// occurrence of `name` in `source`. Falls back to 0 if fewer occurrences exist.
fn find_nth_occurrence(source: &str, name: &str, nth: usize) -> usize {
    let bytes = source.as_bytes();
    let name_bytes: Vec<u8> = name.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let name_len = name_bytes.len();
    let src_len = bytes.len();

    let mut count = 0usize;
    let mut i = 0usize;

    while i + name_len <= src_len {
        let before_ok = i == 0 || {
            let b = bytes[i - 1];
            !b.is_ascii_alphanumeric() && b != b'_'
        };

        if before_ok {
            let matches = bytes[i..i + name_len]
                .iter()
                .zip(name_bytes.iter())
                .all(|(&a, &b)| a.to_ascii_uppercase() == b);

            if matches {
                let after = i + name_len;
                let after_ok = after >= src_len || {
                    let b = bytes[after];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };

                if after_ok {
                    if count == nth {
                        return i;
                    }
                    count += 1;
                }
            }
        }

        i += 1;
    }

    0
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
