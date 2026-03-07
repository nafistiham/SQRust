use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct RecursiveCte;

impl Rule for RecursiveCte {
    fn name(&self) -> &'static str {
        "Lint/RecursiveCte"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST is incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let source = &ctx.source;
        let source_upper = source.to_uppercase();
        let mut search_from = 0usize;

        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                collect_from_query(q, source, &source_upper, &mut search_from, self.name(), &mut diags);
            }
        }

        diags
    }
}

/// Recursively walks a query, flagging every WITH RECURSIVE it encounters.
fn collect_from_query(
    query: &Query,
    source: &str,
    source_upper: &str,
    search_from: &mut usize,
    rule_name: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        if with.recursive {
            let (line, col) =
                find_keyword_position(source, source_upper, "WITH RECURSIVE", search_from);
            diags.push(Diagnostic {
                rule: rule_name,
                message:
                    "WITH RECURSIVE CTE may loop indefinitely; ensure a correct termination condition"
                        .to_string(),
                line,
                col,
            });
        } else {
            // Non-recursive WITH — advance past its WITH keyword.
            advance_past_keyword(source, source_upper, "WITH", search_from);
        }

        // Walk each CTE's inner query for nested recursive CTEs.
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, source, source_upper, search_from, rule_name, diags);
        }
    }

    // Walk the body of the query (may contain subqueries / derived tables).
    collect_from_set_expr(&query.body, source, source_upper, search_from, rule_name, diags);
}

fn collect_from_set_expr(
    expr: &SetExpr,
    source: &str,
    source_upper: &str,
    search_from: &mut usize,
    rule_name: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(select) => {
            collect_from_select(select, source, source_upper, search_from, rule_name, diags);
        }
        SetExpr::Query(inner) => {
            collect_from_query(inner, source, source_upper, search_from, rule_name, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            collect_from_set_expr(left, source, source_upper, search_from, rule_name, diags);
            collect_from_set_expr(right, source, source_upper, search_from, rule_name, diags);
        }
        _ => {}
    }
}

fn collect_from_select(
    select: &Select,
    source: &str,
    source_upper: &str,
    search_from: &mut usize,
    rule_name: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    // Walk FROM clause — derived tables may contain WITH RECURSIVE.
    for twj in &select.from {
        collect_from_table_factor(&twj.relation, source, source_upper, search_from, rule_name, diags);
        for join in &twj.joins {
            collect_from_table_factor(&join.relation, source, source_upper, search_from, rule_name, diags);
        }
    }

    // Walk SELECT projection for subqueries.
    for item in &select.projection {
        let expr = match item {
            SelectItem::UnnamedExpr(e) => Some(e),
            SelectItem::ExprWithAlias { expr: e, .. } => Some(e),
            _ => None,
        };
        if let Some(e) = expr {
            collect_from_expr(e, source, source_upper, search_from, rule_name, diags);
        }
    }

    // Walk WHERE.
    if let Some(selection) = &select.selection {
        collect_from_expr(selection, source, source_upper, search_from, rule_name, diags);
    }
}

fn collect_from_table_factor(
    factor: &TableFactor,
    source: &str,
    source_upper: &str,
    search_from: &mut usize,
    rule_name: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        collect_from_query(subquery, source, source_upper, search_from, rule_name, diags);
    }
}

fn collect_from_expr(
    expr: &Expr,
    source: &str,
    source_upper: &str,
    search_from: &mut usize,
    rule_name: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Subquery(q) | Expr::InSubquery { subquery: q, .. } | Expr::Exists { subquery: q, .. } => {
            collect_from_query(q, source, source_upper, search_from, rule_name, diags);
        }
        _ => {}
    }
}

/// Finds the 1-indexed (line, col) of the next word-boundary occurrence of
/// `keyword` (already uppercased) in `source_upper` starting from `search_from`.
/// Updates `search_from` to just past the match. Falls back to (1, 1).
fn find_keyword_position(
    source: &str,
    source_upper: &str,
    keyword: &str,
    search_from: &mut usize,
) -> (usize, usize) {
    let (line, col, new_from) = find_keyword_inner(source, source_upper, keyword, *search_from);
    *search_from = new_from;
    (line, col)
}

/// Advance `search_from` past the next word-boundary occurrence of `keyword`
/// without emitting a diagnostic.
fn advance_past_keyword(
    source: &str,
    source_upper: &str,
    keyword: &str,
    search_from: &mut usize,
) {
    let (_, _, new_from) = find_keyword_inner(source, source_upper, keyword, *search_from);
    *search_from = new_from;
}

/// Core search: returns (line, col, next_search_from).
/// Requires `keyword` to be already uppercased.
fn find_keyword_inner(
    source: &str,
    source_upper: &str,
    keyword: &str,
    start: usize,
) -> (usize, usize, usize) {
    let kw_len = keyword.len();
    let bytes = source_upper.as_bytes();
    let text_len = bytes.len();

    let mut pos = start;
    while pos < text_len {
        let Some(rel) = source_upper[pos..].find(keyword) else {
            break;
        };
        let abs = pos + rel;

        // Word-boundary check: character before keyword.
        let before_ok = abs == 0
            || {
                let b = bytes[abs - 1];
                !b.is_ascii_alphanumeric() && b != b'_'
            };
        // Word-boundary check: character after keyword.
        let after = abs + kw_len;
        let after_ok = after >= text_len
            || {
                let b = bytes[after];
                !b.is_ascii_alphanumeric() && b != b'_'
            };

        if before_ok && after_ok {
            let (line, col) = offset_to_line_col(source, abs);
            return (line, col, after);
        }
        pos = abs + 1;
    }

    (1, 1, start)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
