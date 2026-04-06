use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{GroupByExpr, Query, Select, SetExpr, Statement};

pub struct DuplicateGroupByColumn;

impl Rule for DuplicateGroupByColumn {
    fn name(&self) -> &'static str {
        "Structure/DuplicateGroupByColumn"
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

    check_set_expr(&query.body, ctx, diags);
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => {
            check_select(sel, ctx, diags);
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

fn check_select(sel: &Select, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let GroupByExpr::Expressions(exprs, _) = &sel.group_by {
        let mut seen: Vec<String> = Vec::new();
        for expr in exprs {
            let normalized = format!("{}", expr).to_lowercase();
            if seen.contains(&normalized) {
                let (line, col) = find_group_by_pos(&ctx.source);
                diags.push(Diagnostic {
                    rule: "Structure/DuplicateGroupByColumn",
                    message: format!(
                        "GROUP BY contains duplicate column '{}'; remove the redundant grouping key",
                        normalized
                    ),
                    line,
                    col,
                });
                // Report once per GROUP BY clause (first duplicate found).
                break;
            }
            seen.push(normalized);
        }
    }
    // GroupByExpr::All — skip; not a column list we can meaningfully deduplicate.
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the first `GROUP BY` keyword in `source` (case-insensitive,
/// word-boundary) and return a 1-indexed (line, col) pair.
/// Falls back to (1, 1) if not found.
fn find_group_by_pos(source: &str) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Match "GROUP" case-insensitively (5 chars).
        if i + 5 <= len
            && bytes[i].eq_ignore_ascii_case(&b'G')
            && bytes[i + 1].eq_ignore_ascii_case(&b'R')
            && bytes[i + 2].eq_ignore_ascii_case(&b'O')
            && bytes[i + 3].eq_ignore_ascii_case(&b'U')
            && bytes[i + 4].eq_ignore_ascii_case(&b'P')
        {
            let after_group = i + 5;
            if after_group < len && is_word_char(bytes[after_group]) {
                i += 1;
                continue;
            }

            // Skip whitespace between GROUP and BY.
            let mut j = after_group;
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
