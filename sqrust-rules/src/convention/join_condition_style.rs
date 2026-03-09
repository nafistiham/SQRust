use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{BinaryOperator, Expr, Query, Select, SetExpr, Statement, TableFactor};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct JoinConditionStyle;

impl Rule for JoinConditionStyle {
    fn name(&self) -> &'static str {
        "Convention/JoinConditionStyle"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        let mut count = 0usize;
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, ctx, &mut count, &mut diags);
            }
        }
        diags
    }
}

fn check_query(q: &Query, ctx: &FileContext, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &q.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, count, diags);
        }
    }
    check_set_expr(&q.body, ctx, count, diags);
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, ctx, count, diags),
        SetExpr::Query(q) => check_query(q, ctx, count, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, count, diags);
            check_set_expr(right, ctx, count, diags);
        }
        _ => {}
    }
}

fn check_select(sel: &Select, ctx: &FileContext, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    for twj in &sel.from {
        recurse_factor(&twj.relation, ctx, count, diags);
        for join in &twj.joins {
            recurse_factor(&join.relation, ctx, count, diags);
        }
    }
    if let Some(where_expr) = &sel.selection {
        collect_cross_table_eq(where_expr, ctx, count, diags);
    }
}

fn recurse_factor(tf: &TableFactor, ctx: &FileContext, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, ctx, count, diags);
    }
}

fn collect_cross_table_eq(expr: &Expr, ctx: &FileContext, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::BinaryOp { left, op, right } => {
            if matches!(op, BinaryOperator::Eq) {
                if let (Expr::CompoundIdentifier(l_parts), Expr::CompoundIdentifier(r_parts)) =
                    (left.as_ref(), right.as_ref())
                {
                    if l_parts.len() >= 2 && r_parts.len() >= 2 {
                        let l_table = l_parts[0].value.to_lowercase();
                        let r_table = r_parts[0].value.to_lowercase();
                        if l_table != r_table {
                            let occ = *count;
                            *count += 1;
                            if let Some(offset) = find_nth_word(&ctx.source, &l_parts[0].value, occ) {
                                let (line, col) = offset_to_line_col(&ctx.source, offset);
                                diags.push(Diagnostic {
                                    rule: "Convention/JoinConditionStyle",
                                    message: "Join condition found in WHERE clause; move it to the ON clause".to_string(),
                                    line,
                                    col,
                                });
                            }
                            return;
                        }
                    }
                }
            }
            collect_cross_table_eq(left, ctx, count, diags);
            collect_cross_table_eq(right, ctx, count, diags);
        }
        Expr::Nested(inner) => collect_cross_table_eq(inner, ctx, count, diags),
        _ => {}
    }
}

fn find_nth_word(source: &str, word: &str, nth: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let word_upper: Vec<u8> = word.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let wlen = word_upper.len();
    let len = bytes.len();
    let skip = SkipMap::build(source);
    let mut count = 0;
    let mut i = 0;
    while i + wlen <= len {
        if !skip.is_code(i) {
            i += 1;
            continue;
        }
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }
        let matches = bytes[i..i + wlen]
            .iter()
            .zip(word_upper.iter())
            .all(|(&a, &b)| a.to_ascii_uppercase() == b);
        if matches {
            let end = i + wlen;
            let after_ok = end >= len || !is_word_char(bytes[end]);
            if after_ok && (i..end).all(|k| skip.is_code(k)) {
                if count == nth {
                    return Some(i);
                }
                count += 1;
                i += wlen;
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
