use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SetExpr, Statement, With};
use std::collections::HashMap;

pub struct OrInsteadOfIn;

impl Rule for OrInsteadOfIn {
    fn name(&self) -> &'static str {
        "Convention/OrInsteadOfIn"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            check_stmt(stmt, &ctx.source, self.name(), &mut diags);
        }
        diags
    }
}

fn check_stmt(stmt: &Statement, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match stmt {
        Statement::Query(q) => check_query(q, src, rule, diags),
        _ => {}
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
    if let Some(expr) = &sel.selection {
        check_expr_for_or_chains(expr, src, rule, diags);
    }
    if let Some(expr) = &sel.having {
        check_expr_for_or_chains(expr, src, rule, diags);
    }
}

fn check_expr_for_or_chains(expr: &Expr, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    // Collect the full OR chain at this level
    let mut equalities: Vec<(String, usize)> = Vec::new(); // (col_name, source_offset)
    collect_or_equalities(expr, &mut equalities, src);

    if equalities.len() >= 2 {
        // Group by column name
        let mut counts: HashMap<&str, Vec<usize>> = HashMap::new();
        for (col, off) in &equalities {
            counts.entry(col.as_str()).or_default().push(*off);
        }
        for (col, offsets) in &counts {
            if offsets.len() >= 3 {
                let off = offsets[0];
                let (line, col_pos) = offset_to_line_col(src, off);
                diags.push(Diagnostic {
                    rule,
                    message: format!(
                        "Column '{}' appears in {} OR equality conditions; use IN() instead",
                        col, offsets.len()
                    ),
                    line,
                    col: col_pos,
                });
            }
        }
        return; // processed this level
    }

    // Recurse into sub-expressions (non-OR operators)
    match expr {
        Expr::BinaryOp { left, right, .. } => {
            // If not all OR, recurse into branches
            if equalities.is_empty() {
                check_expr_for_or_chains(left, src, rule, diags);
                check_expr_for_or_chains(right, src, rule, diags);
            }
        }
        Expr::Nested(inner) => check_expr_for_or_chains(inner, src, rule, diags),
        Expr::UnaryOp { expr: inner, .. } => check_expr_for_or_chains(inner, src, rule, diags),
        _ => {}
    }
}

/// Recursively collects (column_name, offset) for each `col = literal` in an OR chain.
/// Only collects from BinaryOp(Or) chains; stops at non-Or operators.
fn collect_or_equalities(expr: &Expr, out: &mut Vec<(String, usize)>, src: &str) {
    use sqlparser::ast::BinaryOperator;
    match expr {
        Expr::BinaryOp { left, op: BinaryOperator::Or, right } => {
            collect_or_equalities(left, out, src);
            collect_or_equalities(right, out, src);
        }
        Expr::BinaryOp { left, op: BinaryOperator::Eq, right } => {
            // Check for col = literal pattern
            let col_name = expr_to_col_name(left)
                .or_else(|| expr_to_col_name(right));
            if let Some(name) = col_name {
                // Find position of the column name in source
                if let Some(off) = find_word_in_source(src, &name, 0) {
                    out.push((name, off));
                }
            }
        }
        Expr::Nested(inner) => collect_or_equalities(inner, out, src),
        _ => {}
    }
}

fn expr_to_col_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Identifier(i) => Some(i.value.to_lowercase()),
        Expr::CompoundIdentifier(parts) => {
            // Use the full dotted path for qualified refs
            Some(parts.iter().map(|p| p.value.to_lowercase()).collect::<Vec<_>>().join("."))
        }
        _ => None,
    }
}

fn find_word_in_source(src: &str, word: &str, start: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let wbytes = word.as_bytes();
    let wlen = wbytes.len();
    if wlen == 0 || start + wlen > bytes.len() {
        return None;
    }
    let mut i = start;
    while i + wlen <= bytes.len() {
        if bytes[i..i + wlen].eq_ignore_ascii_case(wbytes) {
            let before_ok = i == 0 || !is_word_char_plain(bytes[i - 1]);
            let after_ok = i + wlen >= bytes.len() || !is_word_char_plain(bytes[i + wlen]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_word_char_plain(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
