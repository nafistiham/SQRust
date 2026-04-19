use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor, With};

pub struct SelectStar;

impl Rule for SelectStar {
    fn name(&self) -> &'static str {
        "Convention/SelectStar"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        let mut search_from = 0usize;

        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, &ctx.source, self.name(), &mut diags, &mut search_from);
            }
        }

        diags
    }
}

fn check_query(
    q: &Query,
    src: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    search_from: &mut usize,
) {
    if let Some(With { cte_tables, .. }) = &q.with {
        for cte in cte_tables {
            check_query(&cte.query, src, rule, diags, search_from);
        }
    }
    check_set_expr(&q.body, src, rule, diags, search_from);
}

fn check_set_expr(
    expr: &SetExpr,
    src: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    search_from: &mut usize,
) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, src, rule, diags, search_from),
        SetExpr::Query(inner) => check_query(inner, src, rule, diags, search_from),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, src, rule, diags, search_from);
            check_set_expr(right, src, rule, diags, search_from);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    src: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    search_from: &mut usize,
) {
    let has_wildcard = sel.projection.iter().any(|item| {
        matches!(
            item,
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
        )
    });

    if has_wildcard {
        let off = find_keyword_from(src, "select", *search_from);
        let (line, col) = match off {
            Some(o) => {
                *search_from = o + 6;
                offset_to_line_col(src, o)
            }
            None => (1, 1),
        };
        diags.push(Diagnostic {
            rule,
            message: "Avoid SELECT *; list columns explicitly".to_string(),
            line,
            col,
        });
    } else if let Some(o) = find_keyword_from(src, "select", *search_from) {
        *search_from = o + 6;
    }

    // Recurse into subqueries in FROM and JOINs
    for twj in &sel.from {
        check_table_factor(&twj.relation, src, rule, diags, search_from);
        for join in &twj.joins {
            check_table_factor(&join.relation, src, rule, diags, search_from);
        }
    }

    // Recurse into subqueries in WHERE
    if let Some(where_expr) = &sel.selection {
        walk_expr(where_expr, src, rule, diags, search_from);
    }

    // Recurse into subqueries in HAVING
    if let Some(having_expr) = &sel.having {
        walk_expr(having_expr, src, rule, diags, search_from);
    }
}

fn check_table_factor(
    tf: &TableFactor,
    src: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    search_from: &mut usize,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, src, rule, diags, search_from);
    }
}

/// Walk an expression looking for subquery nodes.
fn walk_expr(
    expr: &Expr,
    src: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    search_from: &mut usize,
) {
    match expr {
        Expr::Subquery(q) => check_query(q, src, rule, diags, search_from),
        Expr::InSubquery { subquery, expr: inner, .. } => {
            walk_expr(inner, src, rule, diags, search_from);
            check_query(subquery, src, rule, diags, search_from);
        }
        Expr::Exists { subquery, .. } => {
            check_query(subquery, src, rule, diags, search_from);
        }
        Expr::BinaryOp { left, right, .. } => {
            walk_expr(left, src, rule, diags, search_from);
            walk_expr(right, src, rule, diags, search_from);
        }
        Expr::UnaryOp { expr: inner, .. } => {
            walk_expr(inner, src, rule, diags, search_from);
        }
        Expr::Nested(inner) => {
            walk_expr(inner, src, rule, diags, search_from);
        }
        _ => {}
    }
}

/// Find the byte offset of `keyword` (case-insensitive, word-boundary) at or after `from`.
fn find_keyword_from(src: &str, keyword: &str, from: usize) -> Option<usize> {
    let kw_len = keyword.len();
    let src_bytes = src.as_bytes();
    let len = src_bytes.len();
    let kw_lower: Vec<u8> = keyword.bytes().map(|b| b.to_ascii_lowercase()).collect();
    let mut i = from;
    while i + kw_len <= len {
        if src_bytes[i..i + kw_len]
            .iter()
            .zip(kw_lower.iter())
            .all(|(a, b)| a.to_ascii_lowercase() == *b)
        {
            let before_ok = i == 0 || !is_word_char(src_bytes[i - 1]);
            let after_ok = i + kw_len >= len || !is_word_char(src_bytes[i + kw_len]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
