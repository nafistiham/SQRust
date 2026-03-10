use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, With};
use std::collections::HashSet;

pub struct ColumnAliasInWhere;

impl Rule for ColumnAliasInWhere {
    fn name(&self) -> &'static str {
        "Lint/ColumnAliasInWhere"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            check_stmt(stmt, &ctx.source, "Lint/ColumnAliasInWhere", &mut diags);
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
    // Collect SELECT aliases
    let mut aliases: HashSet<String> = HashSet::new();
    for item in &sel.projection {
        if let SelectItem::ExprWithAlias { alias, .. } = item {
            aliases.insert(alias.value.to_lowercase());
        }
    }

    if aliases.is_empty() {
        return;
    }

    // Walk WHERE for identifiers matching aliases
    if let Some(where_expr) = &sel.selection {
        let start_offset = find_where_offset(src);
        find_alias_refs(where_expr, &aliases, src, rule, diags, start_offset);
    }
}

fn find_where_offset(src: &str) -> usize {
    let bytes = src.as_bytes();
    let kw = b"WHERE";
    let mut i = 0;
    while i + 5 <= bytes.len() {
        if bytes[i..i + 5].eq_ignore_ascii_case(kw) {
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + 5 >= bytes.len() || !is_word_char(bytes[i + 5]);
            if before_ok && after_ok {
                return i;
            }
        }
        i += 1;
    }
    0
}

fn find_alias_refs(
    expr: &Expr,
    aliases: &HashSet<String>,
    src: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    start_offset: usize,
) {
    match expr {
        Expr::Identifier(ident) => {
            let lower = ident.value.to_lowercase();
            if aliases.contains(&lower) {
                if let Some(off) = find_word_in_source(src, &ident.value, start_offset) {
                    let (line, col) = offset_to_line_col(src, off);
                    diags.push(Diagnostic {
                        rule,
                        message: format!(
                            "Column alias '{}' is used in WHERE clause; aliases are not available in WHERE (evaluated before SELECT)",
                            ident.value
                        ),
                        line,
                        col,
                    });
                }
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            find_alias_refs(left, aliases, src, rule, diags, start_offset);
            find_alias_refs(right, aliases, src, rule, diags, start_offset);
        }
        Expr::UnaryOp { expr, .. } | Expr::Nested(expr) => {
            find_alias_refs(expr, aliases, src, rule, diags, start_offset);
        }
        Expr::Between { expr, low, high, .. } => {
            find_alias_refs(expr, aliases, src, rule, diags, start_offset);
            find_alias_refs(low, aliases, src, rule, diags, start_offset);
            find_alias_refs(high, aliases, src, rule, diags, start_offset);
        }
        Expr::InList { expr, list, .. } => {
            find_alias_refs(expr, aliases, src, rule, diags, start_offset);
            for e in list {
                find_alias_refs(e, aliases, src, rule, diags, start_offset);
            }
        }
        Expr::IsNull(e) | Expr::IsNotNull(e) => {
            find_alias_refs(e, aliases, src, rule, diags, start_offset);
        }
        Expr::Like { expr, pattern, .. } | Expr::ILike { expr, pattern, .. } => {
            find_alias_refs(expr, aliases, src, rule, diags, start_offset);
            find_alias_refs(pattern, aliases, src, rule, diags, start_offset);
        }
        _ => {}
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
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + wlen >= bytes.len() || !is_word_char(bytes[i + wlen]);
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
