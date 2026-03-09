use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{JoinOperator, Query, Select, SetExpr, Statement, TableFactor};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct LeftJoin;

impl Rule for LeftJoin {
    fn name(&self) -> &'static str {
        "Convention/LeftJoin"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        let mut count = 0usize;
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, &ctx.source, &mut count, &mut diags);
            }
        }
        diags
    }
}

fn check_query(q: &Query, src: &str, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &q.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, src, count, diags);
        }
    }
    check_set_expr(&q.body, src, count, diags);
}

fn check_set_expr(expr: &SetExpr, src: &str, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, src, count, diags),
        SetExpr::Query(q) => check_query(q, src, count, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, src, count, diags);
            check_set_expr(right, src, count, diags);
        }
        _ => {}
    }
}

fn check_select(sel: &Select, src: &str, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    for twj in &sel.from {
        recurse_factor(&twj.relation, src, count, diags);
        for join in &twj.joins {
            recurse_factor(&join.relation, src, count, diags);
            if is_right_join(&join.join_operator) {
                let occ = *count;
                *count += 1;
                if let Some(offset) = find_nth_keyword(src, b"RIGHT", occ) {
                    let (line, col) = offset_to_line_col(src, offset);
                    diags.push(Diagnostic {
                        rule: "Convention/LeftJoin",
                        message: "Prefer LEFT JOIN over RIGHT JOIN; rewrite from the other table's perspective".to_string(),
                        line,
                        col,
                    });
                }
            }
        }
    }
}

fn recurse_factor(tf: &TableFactor, src: &str, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, src, count, diags);
    }
}

fn is_right_join(op: &JoinOperator) -> bool {
    matches!(
        op,
        JoinOperator::RightOuter(_) | JoinOperator::RightSemi(_) | JoinOperator::RightAnti(_)
    )
}

fn find_nth_keyword(source: &str, keyword: &[u8], nth: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let kw_len = keyword.len();
    let len = bytes.len();
    let skip = SkipMap::build(source);
    let mut count = 0;
    let mut i = 0;
    while i + kw_len <= len {
        if !skip.is_code(i) {
            i += 1;
            continue;
        }
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(keyword.iter())
            .all(|(&a, &b)| a.to_ascii_uppercase() == b.to_ascii_uppercase());
        if matches {
            let end = i + kw_len;
            let after_ok = end >= len || !is_word_char(bytes[end]);
            let all_code = (i..end).all(|k| skip.is_code(k));
            if after_ok && all_code {
                if count == nth {
                    return Some(i);
                }
                count += 1;
                i += kw_len;
                continue;
            }
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
