use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SetExpr, Statement};

use crate::capitalisation::{is_word_char, SkipMap};

pub struct TooManyUnions {
    /// Maximum number of UNION/INTERSECT/EXCEPT set operations allowed in a
    /// query. Queries with more set operations than this are flagged.
    pub max_unions: usize,
}

impl Default for TooManyUnions {
    fn default() -> Self {
        TooManyUnions { max_unions: 3 }
    }
}

impl Rule for TooManyUnions {
    fn name(&self) -> &'static str {
        "TooManyUnions"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(query, self.max_unions, ctx, &mut diags);
            }
        }

        diags
    }
}

// ── AST walking ───────────────────────────────────────────────────────────────

fn check_query(
    query: &Query,
    max: usize,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    // Visit CTEs.
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, max, ctx, diags);
        }
    }

    let n = count_set_ops(&query.body);
    if n > max {
        // Find the first set operation keyword in source.
        let (line, col) = find_first_set_op_pos(&ctx.source);
        diags.push(Diagnostic {
            rule: "TooManyUnions",
            message: format!(
                "Query has {n} UNION operations, exceeding the maximum of {max}",
                n = n,
                max = max,
            ),
            line,
            col,
        });
    }
}

// ── set operation counting ────────────────────────────────────────────────────

/// Count the number of SetOperation nodes in a SetExpr tree.
/// Each UNION / UNION ALL / INTERSECT / EXCEPT node adds 1 to the count.
fn count_set_ops(body: &SetExpr) -> usize {
    match body {
        SetExpr::SetOperation { left, right, .. } => {
            1 + count_set_ops(left) + count_set_ops(right)
        }
        SetExpr::Query(q) => count_set_ops(&q.body),
        _ => 0,
    }
}

// ── keyword position helper ───────────────────────────────────────────────────

/// Find the position of the first set operation keyword (UNION, INTERSECT, or
/// EXCEPT) in `source`. Returns 1-indexed (line, col). Falls back to (1, 1).
fn find_first_set_op_pos(source: &str) -> (usize, usize) {
    let keywords = ["UNION", "INTERSECT", "EXCEPT"];

    let mut best: Option<usize> = None;

    for kw in &keywords {
        if let Some(offset) = find_keyword_offset(source, kw) {
            best = Some(match best {
                None => offset,
                Some(prev) => prev.min(offset),
            });
        }
    }

    match best {
        Some(offset) => line_col(source, offset),
        None => (1, 1),
    }
}

/// Find the byte offset of the first occurrence of `keyword` as a whole word
/// (case-insensitive, outside strings/comments). Returns None if not found.
fn find_keyword_offset(source: &str, keyword: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let skip_map = SkipMap::build(source);
    let kw_upper: Vec<u8> = keyword.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let kw_len = kw_upper.len();

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
                return Some(i);
            }
        }

        i += 1;
    }

    None
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
