use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, Select, SetExpr, Statement, TableFactor, TableWithJoins, With};
use std::collections::HashMap;

pub struct DuplicateJoin;

impl Rule for DuplicateJoin {
    fn name(&self) -> &'static str {
        "Lint/DuplicateJoin"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            check_stmt(stmt, &ctx.source, "Lint/DuplicateJoin", &mut diags);
        }
        diags
    }
}

fn check_stmt(stmt: &Statement, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if let Statement::Query(q) = stmt {
        check_query(q, src, rule, diags);
    }
}

fn check_query(q: &Query, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if let Some(With { cte_tables, .. }) = &q.with {
        for cte in cte_tables {
            check_query(&cte.query, src, rule, diags);
        }
    }
    check_set_expr(&q.body, src, rule, diags);
}

fn check_set_expr(body: &SetExpr, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match body {
        SetExpr::Select(s) => check_select(s, src, rule, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, src, rule, diags);
            check_set_expr(right, src, rule, diags);
        }
        SetExpr::Query(q) => check_query(q, src, rule, diags),
        _ => {}
    }
}

fn check_select(sel: &Select, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    for twj in &sel.from {
        check_table_with_joins(twj, src, rule, diags);
    }
}

fn check_table_with_joins(
    twj: &TableWithJoins,
    src: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    // Collect all table names (lowercased full name) with first occurrence offset
    let mut seen: HashMap<String, usize> = HashMap::new();

    // Main table
    if let Some((name, off)) = table_factor_name(&twj.relation, src) {
        seen.insert(name, off);
    }

    // Recurse into subqueries in main table
    check_factor_subqueries(&twj.relation, src, rule, diags);

    // JOINs
    let mut flagged = false;
    for join in &twj.joins {
        check_factor_subqueries(&join.relation, src, rule, diags);
        if let Some((name, off)) = table_factor_name(&join.relation, src) {
            if seen.contains_key(&name) && !flagged {
                let (line, col) = offset_to_line_col(src, off);
                diags.push(Diagnostic {
                    rule,
                    message: format!(
                        "Table '{}' is joined more than once in the same FROM clause",
                        name
                    ),
                    line,
                    col,
                });
                flagged = true;
            } else {
                seen.insert(name, off);
            }
        }
    }
}

fn table_factor_name(tf: &TableFactor, src: &str) -> Option<(String, usize)> {
    match tf {
        TableFactor::Table { name, .. } => {
            let full_name = name
                .0
                .iter()
                .map(|i| i.value.to_lowercase())
                .collect::<Vec<_>>()
                .join(".");
            let last = name.0.last()?.value.clone();
            let off = find_word_in_source(src, &last, 0)?;
            Some((full_name, off))
        }
        _ => None,
    }
}

fn check_factor_subqueries(
    tf: &TableFactor,
    src: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, src, rule, diags);
    }
}

fn find_word_in_source(src: &str, word: &str, start: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let wbytes = word.as_bytes();
    let wlen = wbytes.len();
    if wlen == 0 {
        return None;
    }
    let mut i = start;
    while i + wlen <= bytes.len() {
        if bytes[i..i + wlen].eq_ignore_ascii_case(wbytes) {
            let before_ok = i == 0 || !is_wc(bytes[i - 1]);
            let after_ok = i + wlen >= bytes.len() || !is_wc(bytes[i + wlen]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_wc(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
