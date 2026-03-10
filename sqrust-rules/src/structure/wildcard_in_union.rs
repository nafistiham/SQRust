use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SelectItem, SetExpr, Statement, With};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct WildcardInUnion;

impl Rule for WildcardInUnion {
    fn name(&self) -> &'static str {
        "Structure/WildcardInUnion"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, &ctx.source, self.name(), &mut diags, false, 0);
            }
        }
        diags
    }
}

/// `in_union` is true when this SetExpr is a branch of a UNION/INTERSECT/EXCEPT.
/// `search_from` is the byte offset from which to start searching for SELECT/* in source.
fn check_query(
    q: &Query,
    src: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    in_union: bool,
    search_from: usize,
) {
    if let Some(With { cte_tables, .. }) = &q.with {
        for cte in cte_tables {
            check_query(&cte.query, src, rule, diags, false, 0);
        }
    }
    check_set_expr(&q.body, src, rule, diags, in_union, search_from);
}

fn check_set_expr(
    body: &SetExpr,
    src: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    in_union: bool,
    search_from: usize,
) {
    match body {
        SetExpr::Select(sel) => {
            if in_union {
                let has_wildcard = sel.projection.iter().any(|item| {
                    matches!(
                        item,
                        SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _)
                    )
                });
                if has_wildcard {
                    // Find the SELECT keyword starting from search_from, then find * after it
                    if let Some(off) = find_star_in_select(src, search_from) {
                        let (line, col) = offset_to_line_col(src, off);
                        diags.push(Diagnostic {
                            rule,
                            message: "SELECT * in a UNION/INTERSECT/EXCEPT branch is fragile; list columns explicitly".to_string(),
                            line,
                            col,
                        });
                    }
                }
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            // Find the split point between left and right branches.
            // Left branch starts at search_from; right branch starts after the set operator keyword.
            // We pass search_from to left and try to find a good split for right.
            let left_from = search_from;
            check_set_expr(left, src, rule, diags, true, left_from);

            // Estimate the right-branch start offset by finding the set operator keyword
            // (UNION/INTERSECT/EXCEPT) after the left branch.
            let right_from = find_right_branch_start(src, left, search_from).unwrap_or(search_from);
            check_set_expr(right, src, rule, diags, true, right_from);
        }
        SetExpr::Query(q) => check_query(q, src, rule, diags, in_union, search_from),
        _ => {}
    }
}

/// Find the `*` that belongs to the SELECT starting at or after `search_from`.
/// Strategy: find the SELECT keyword from search_from, then find the first `*`
/// after that SELECT keyword (skipping strings/comments).
fn find_star_in_select(src: &str, search_from: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let skip = SkipMap::build(src);

    // Find SELECT keyword at or after search_from
    let select_kw = b"SELECT";
    let kw_len = select_kw.len();
    let mut select_pos = None;
    let mut i = search_from;
    while i + kw_len <= len {
        if skip.is_code(i) {
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            if before_ok {
                let matches = bytes[i..i + kw_len]
                    .iter()
                    .zip(select_kw.iter())
                    .all(|(&a, &b)| a.to_ascii_uppercase() == b);
                if matches {
                    let end = i + kw_len;
                    let after_ok = end >= len || !is_word_char(bytes[end]);
                    if after_ok {
                        select_pos = Some(i);
                        break;
                    }
                }
            }
        }
        i += 1;
    }

    let start = select_pos? + kw_len;

    // Find FROM keyword after SELECT (marks end of projection list)
    let from_kw = b"FROM";
    let from_len = from_kw.len();
    let mut from_pos = None;
    let mut j = start;
    while j + from_len <= len {
        if skip.is_code(j) {
            let before_ok = j == 0 || !is_word_char(bytes[j - 1]);
            if before_ok {
                let matches = bytes[j..j + from_len]
                    .iter()
                    .zip(from_kw.iter())
                    .all(|(&a, &b)| a.to_ascii_uppercase() == b);
                if matches {
                    let end = j + from_len;
                    let after_ok = end >= len || !is_word_char(bytes[end]);
                    if after_ok {
                        from_pos = Some(j);
                        break;
                    }
                }
            }
        }
        j += 1;
    }

    // Search for `*` between SELECT and FROM (the projection list)
    let search_end = from_pos.unwrap_or(len);
    let mut k = start;
    while k < search_end {
        if skip.is_code(k) && bytes[k] == b'*' {
            return Some(k);
        }
        k += 1;
    }

    None
}

/// Estimate the byte offset where the right branch of a set operation starts.
/// We look for UNION/INTERSECT/EXCEPT after an approximation of where the left branch ends.
fn find_right_branch_start(src: &str, left: &SetExpr, search_from: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let skip = SkipMap::build(src);

    // Approximate the end of the left branch by counting SELECT/FROM tokens.
    // Simpler approach: find the set operator keyword (UNION/INTERSECT/EXCEPT) after search_from,
    // then return the position after it + optional ALL/DISTINCT.
    let set_ops: &[&[u8]] = &[b"INTERSECT", b"EXCEPT", b"UNION"];
    let _ = left; // We don't use the AST for position, just scan source

    let mut i = search_from;
    while i < len {
        if !skip.is_code(i) {
            i += 1;
            continue;
        }
        // Check for each set op keyword
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }
        let mut found_op_end = None;
        for kw in set_ops {
            let kw_len = kw.len();
            if i + kw_len <= len {
                let matches = bytes[i..i + kw_len]
                    .iter()
                    .zip(kw.iter())
                    .all(|(&a, &b)| a.to_ascii_uppercase() == b);
                if matches {
                    let end = i + kw_len;
                    let after_ok = end >= len || !is_word_char(bytes[end]);
                    if after_ok {
                        found_op_end = Some(end);
                        break;
                    }
                }
            }
        }
        if let Some(op_end) = found_op_end {
            // Skip optional ALL or DISTINCT after the operator
            let mut j = op_end;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                j += 1;
            }
            // Check for ALL or DISTINCT
            let all_kw = b"ALL";
            let dist_kw = b"DISTINCT";
            if j + 3 <= len && bytes[j..j + 3].eq_ignore_ascii_case(all_kw) {
                let after = j + 3;
                if after >= len || !is_word_char(bytes[after]) {
                    j = after;
                }
            } else if j + 8 <= len && bytes[j..j + 8].eq_ignore_ascii_case(dist_kw) {
                let after = j + 8;
                if after >= len || !is_word_char(bytes[after]) {
                    j = after;
                }
            }
            return Some(j);
        }
        i += 1;
    }

    None
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
