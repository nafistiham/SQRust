use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, SelectItem, SetExpr, Statement};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct SelectOnlyLiterals;

impl Default for SelectOnlyLiterals {
    fn default() -> Self {
        SelectOnlyLiterals
    }
}

impl Rule for SelectOnlyLiterals {
    fn name(&self) -> &'static str {
        "Structure/SelectOnlyLiterals"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // Track which SELECT keyword occurrence to point at for each flagged statement.
        let mut select_occurrence: usize = 0;

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, ctx, &mut select_occurrence, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    ctx: &FileContext,
    select_occurrence: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, select_occurrence, diags);
        }
    }

    check_set_expr(&query.body, ctx, select_occurrence, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    ctx: &FileContext,
    select_occurrence: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => {
            // Only flag if there is no FROM clause and all projected items are literals.
            if sel.from.is_empty() && !sel.projection.is_empty() {
                let all_literals = sel.projection.iter().all(|item| match item {
                    SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                        is_literal(e)
                    }
                    _ => false,
                });

                if all_literals {
                    let (line, col) =
                        find_keyword_pos(&ctx.source, "SELECT", *select_occurrence);
                    diags.push(Diagnostic {
                        rule: "Structure/SelectOnlyLiterals",
                        message:
                            "SELECT of only literal values with no FROM clause is likely a test/debug query"
                                .to_string(),
                        line,
                        col,
                    });
                }
            }

            *select_occurrence += 1;
        }
        SetExpr::Query(inner) => {
            check_query(inner, ctx, select_occurrence, diags);
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, select_occurrence, diags);
            check_set_expr(right, ctx, select_occurrence, diags);
        }
        _ => {}
    }
}

// ── literal detection ─────────────────────────────────────────────────────────

/// Returns `true` only if `expr` is a bare SQL literal value (number, string,
/// boolean, or NULL). Binary expressions, function calls, column references,
/// etc. all return `false`.
fn is_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Value(_))
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the `nth` (0-indexed) occurrence of a keyword (case-insensitive,
/// word-boundary, outside strings/comments) in `source`. Returns a
/// 1-indexed (line, col) pair. Falls back to (1, 1) if not found.
fn find_keyword_pos(source: &str, keyword: &str, nth: usize) -> (usize, usize) {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);
    let kw_upper: Vec<u8> = keyword.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let kw_len = kw_upper.len();

    let mut count = 0usize;
    let mut i = 0;
    while i + kw_len <= len {
        if !skip_map.is_code(i) {
            i += 1;
            continue;
        }

        // Word boundary before.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive match.
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(kw_upper.iter())
            .all(|(a, b)| a.eq_ignore_ascii_case(b));

        if matches {
            // Word boundary after.
            let after = i + kw_len;
            let after_ok = after >= len || !is_word_char(bytes[after]);
            let all_code = (i..i + kw_len).all(|k| skip_map.is_code(k));

            if after_ok && all_code {
                if count == nth {
                    return line_col(source, i);
                }
                count += 1;
            }
        }

        i += 1;
    }

    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
