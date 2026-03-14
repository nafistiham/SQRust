use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SelectItem, SetExpr, Statement, TableFactor};

pub struct SelectStarInCTE;

impl Rule for SelectStarInCTE {
    fn name(&self) -> &'static str {
        "Structure/SelectStarInCTE"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query_for_cte_stars(query, self.name(), &ctx.source, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

/// Walk the query. If it has a WITH clause, check each CTE body for SELECT *.
/// The main query body is NOT checked — only CTE bodies.
fn check_query_for_cte_stars(
    query: &Query,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            let cte_name = cte.alias.name.value.as_str();
            check_set_expr_for_stars(&cte.query.body, cte_name, rule, source, diags);

            // Also recurse into nested CTEs inside this CTE's query.
            if let Some(inner_with) = &cte.query.with {
                for inner_cte in &inner_with.cte_tables {
                    let inner_name = inner_cte.alias.name.value.as_str();
                    check_set_expr_for_stars(
                        &inner_cte.query.body,
                        inner_name,
                        rule,
                        source,
                        diags,
                    );
                }
            }
        }
    }

    // Recurse into the main query body only to find nested subqueries that
    // themselves contain CTEs (e.g. a derived table that uses WITH).
    check_set_expr_for_nested_ctes(&query.body, rule, source, diags);
}

/// Walk a SetExpr checking it (as a CTE body) for SELECT * at any level.
fn check_set_expr_for_stars(
    body: &SetExpr,
    cte_name: &str,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    match body {
        SetExpr::Select(sel) => {
            // Check the projection list for wildcards.
            let has_star = sel.projection.iter().any(|item| {
                matches!(
                    item,
                    SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
                )
            });

            if has_star {
                let (line, col) = find_star_after_select(source, cte_name);
                diags.push(Diagnostic {
                    rule,
                    message: format!(
                        "SELECT * inside CTE '{cte_name}' \
                         — explicitly list columns in CTE definitions \
                         for documentation and refactoring safety"
                    ),
                    line,
                    col,
                });
            }

            // Recurse into subqueries in the FROM clause.
            for twj in &sel.from {
                check_table_factor_for_stars(&twj.relation, cte_name, rule, source, diags);
                for join in &twj.joins {
                    check_table_factor_for_stars(
                        &join.relation,
                        cte_name,
                        rule,
                        source,
                        diags,
                    );
                }
            }
        }
        SetExpr::Query(inner) => {
            check_set_expr_for_stars(&inner.body, cte_name, rule, source, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr_for_stars(left, cte_name, rule, source, diags);
            check_set_expr_for_stars(right, cte_name, rule, source, diags);
        }
        _ => {}
    }
}

/// Walk a SetExpr that is part of the MAIN query body, looking only for nested
/// subqueries that themselves introduce CTEs. CTE bodies inside those nested
/// subqueries are also checked.
fn check_set_expr_for_nested_ctes(
    body: &SetExpr,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    match body {
        SetExpr::Select(sel) => {
            for twj in &sel.from {
                check_table_factor_for_nested_ctes(&twj.relation, rule, source, diags);
                for join in &twj.joins {
                    check_table_factor_for_nested_ctes(
                        &join.relation,
                        rule,
                        source,
                        diags,
                    );
                }
            }
        }
        SetExpr::Query(inner) => {
            check_query_for_cte_stars(inner, rule, source, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr_for_nested_ctes(left, rule, source, diags);
            check_set_expr_for_nested_ctes(right, rule, source, diags);
        }
        _ => {}
    }
}

fn check_table_factor_for_stars(
    tf: &TableFactor,
    cte_name: &str,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_set_expr_for_stars(&subquery.body, cte_name, rule, source, diags);
    }
}

fn check_table_factor_for_nested_ctes(
    tf: &TableFactor,
    rule: &'static str,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query_for_cte_stars(subquery, rule, source, diags);
    }
}

// ── source-text position helpers ──────────────────────────────────────────────

/// Find the position of the `*` that follows `SELECT` inside the CTE named
/// `cte_name`. Searches for the CTE name first, then finds `SELECT * ` after
/// it. Falls back to (1, 1).
fn find_star_after_select(source: &str, cte_name: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let upper = source.to_uppercase();
    let len = bytes.len();

    // Find the CTE name in source (case-insensitive).
    let name_upper = cte_name.to_uppercase();
    let name_len = name_upper.len();

    let mut search = 0usize;
    while search + name_len <= len {
        let Some(rel) = upper[search..].find(name_upper.as_str()) else {
            break;
        };
        let abs = search + rel;

        let before_ok = abs == 0 || !is_word_char(bytes[abs - 1]);
        let after = abs + name_len;
        let after_ok = after >= len || !is_word_char(bytes[after]);

        if before_ok && after_ok {
            // Found the CTE name occurrence. Now search for SELECT * after it.
            if let Some(star_offset) = find_select_star_after(source, &upper, after) {
                return offset_to_line_col(source, star_offset);
            }
        }
        search = abs + 1;
    }

    (1, 1)
}

/// Find the offset of `*` that belongs to a `SELECT *` occurring at or after
/// `start` in source. Returns `None` if not found.
fn find_select_star_after(source: &str, upper: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let kw = b"SELECT";
    let kw_len = kw.len();

    // Find the SELECT keyword.
    let mut i = start;
    let mut select_end = None;
    while i + kw_len <= len {
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if before_ok {
            let matches = upper[i..].starts_with("SELECT");
            if matches {
                let after = i + kw_len;
                let after_ok = after >= len || !is_word_char(bytes[after]);
                if after_ok {
                    select_end = Some(after);
                    break;
                }
            }
        }
        i += 1;
    }

    let sel_end = select_end?;

    // Find `*` between SELECT and FROM (the projection list).
    let from_kw = b"FROM";
    let from_len = from_kw.len();
    let mut from_pos = None;
    let mut j = sel_end;
    while j + from_len <= len {
        let before_ok = j == 0 || !is_word_char(bytes[j - 1]);
        if before_ok {
            let upper_slice = &upper[j..];
            if upper_slice.starts_with("FROM") {
                let after = j + from_len;
                let after_ok = after >= len || !is_word_char(bytes[after]);
                if after_ok {
                    from_pos = Some(j);
                    break;
                }
            }
        }
        j += 1;
    }

    let search_end = from_pos.unwrap_or(len);
    let mut k = sel_end;
    while k < search_end {
        if bytes[k] == b'*' {
            return Some(k);
        }
        k += 1;
    }

    None
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
