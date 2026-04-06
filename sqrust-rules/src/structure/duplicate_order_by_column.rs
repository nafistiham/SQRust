use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SetExpr, Statement};

pub struct DuplicateOrderByColumn;

impl Rule for DuplicateOrderByColumn {
    fn name(&self) -> &'static str {
        "Structure/DuplicateOrderByColumn"
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

fn check_query(query: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    // Visit CTEs first.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, diags);
        }
    }

    // Check ORDER BY on this query level.
    if let Some(order_by) = &query.order_by {
        let mut seen: Vec<String> = Vec::new();
        for order_expr in &order_by.exprs {
            let normalized = format!("{}", order_expr.expr).to_lowercase();
            if seen.contains(&normalized) {
                let (line, col) = find_order_by_pos(&ctx.source);
                diags.push(Diagnostic {
                    rule: "Structure/DuplicateOrderByColumn",
                    message: format!(
                        "ORDER BY contains duplicate column '{}'; remove the redundant sort key",
                        normalized
                    ),
                    line,
                    col,
                });
                // Report once per ORDER BY clause (first duplicate found).
                break;
            }
            seen.push(normalized);
        }
    }

    // Recurse into the body to catch nested subqueries.
    check_set_expr(&query.body, ctx, diags);
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(_) => {
            // No nested queries directly inside a plain Select body that we
            // haven't already covered via check_query at the outer level.
        }
        SetExpr::Query(inner) => {
            check_query(inner, ctx, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, diags);
            check_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the first `ORDER BY` keyword in `source` (case-insensitive,
/// word-boundary) and return a 1-indexed (line, col) pair.
/// Falls back to (1, 1) if not found.
fn find_order_by_pos(source: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Match "ORDER" case-insensitively.
        if i + 5 <= len
            && bytes[i].eq_ignore_ascii_case(&b'O')
            && bytes[i + 1].eq_ignore_ascii_case(&b'R')
            && bytes[i + 2].eq_ignore_ascii_case(&b'D')
            && bytes[i + 3].eq_ignore_ascii_case(&b'E')
            && bytes[i + 4].eq_ignore_ascii_case(&b'R')
        {
            let after_order = i + 5;
            if after_order < len && is_word_char(bytes[after_order]) {
                i += 1;
                continue;
            }

            // Skip whitespace between ORDER and BY.
            let mut j = after_order;
            while j < len
                && (bytes[j] == b' '
                    || bytes[j] == b'\t'
                    || bytes[j] == b'\n'
                    || bytes[j] == b'\r')
            {
                j += 1;
            }

            // Match "BY" case-insensitively.
            if j + 2 <= len
                && bytes[j].eq_ignore_ascii_case(&b'B')
                && bytes[j + 1].eq_ignore_ascii_case(&b'Y')
            {
                let after_by = j + 2;
                if after_by >= len || !is_word_char(bytes[after_by]) {
                    return line_col(source, i);
                }
            }
        }

        i += 1;
    }

    (1, 1)
}

#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
