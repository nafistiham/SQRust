use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, Select, SetExpr, Statement, TableFactor};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct UnusedTableAlias;

impl Rule for UnusedTableAlias {
    fn name(&self) -> &'static str {
        "Lint/UnusedTableAlias"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, ctx, &mut diags);
            }
        }
        diags
    }
}

fn check_query(q: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &q.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, diags);
        }
    }
    check_set_expr(&q.body, ctx, diags);
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, ctx, diags),
        SetExpr::Query(q) => check_query(q, ctx, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, diags);
            check_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(sel: &Select, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    for twj in &sel.from {
        check_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, ctx, diags);
        }
    }
}

fn check_table_factor(tf: &TableFactor, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match tf {
        TableFactor::Table { alias, .. } => {
            if let Some(table_alias) = alias {
                check_alias_used(&table_alias.name.value, ctx, diags);
            }
        }
        TableFactor::Derived { subquery, alias, .. } => {
            check_query(subquery, ctx, diags);
            if let Some(table_alias) = alias {
                check_alias_used(&table_alias.name.value, ctx, diags);
            }
        }
        _ => {}
    }
}

fn check_alias_used(alias: &str, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    let source = &ctx.source;
    let source_upper = source.to_uppercase();
    let alias_upper = alias.to_uppercase();

    // Find position of alias definition: look for `AS <alias>` in the source.
    let def_pos = match find_alias_definition(source, alias) {
        Some(p) => p,
        // Fallback: find any whole-word occurrence.
        None => match find_word_position(source, alias) {
            Some(p) => p,
            None => return,
        },
    };

    // Check if `alias.` appears anywhere in the entire source (the usage
    // may appear before or after the definition in the SQL text).
    let qualifier = format!("{}.", alias_upper);
    if source_upper.contains(&qualifier) {
        return;
    }

    let (line, col) = offset_to_line_col(source, def_pos);
    diags.push(Diagnostic {
        rule: "Lint/UnusedTableAlias",
        message: format!("Table alias '{}' is defined but never used as a qualifier", alias),
        line,
        col,
    });
}

/// Finds the byte position of the alias name in an `AS <alias>` clause,
/// searching case-insensitively. Returns the position of the alias name itself.
fn find_alias_definition(source: &str, alias: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let alias_upper: Vec<u8> = alias.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let alias_len = alias_upper.len();
    let skip = SkipMap::build(source);

    // Pattern: `AS` (word-bounded, in code) followed by whitespace then `alias` (word-bounded).
    let mut i = 0;
    while i + 2 <= len {
        if !skip.is_code(i) {
            i += 1;
            continue;
        }
        // Try to match `AS` at position i.
        if i + 2 <= len
            && bytes[i].to_ascii_uppercase() == b'A'
            && bytes[i + 1].to_ascii_uppercase() == b'S'
        {
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_as = i + 2;
            let after_ok = after_as >= len || !is_word_char(bytes[after_as]);
            if before_ok && after_ok && skip.is_code(i + 1) {
                // Skip whitespace after AS.
                let mut j = after_as;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                // Try to match alias at j.
                if j + alias_len <= len {
                    let alias_before_ok = j == 0 || !is_word_char(bytes[j - 1]);
                    let matches = bytes[j..j + alias_len]
                        .iter()
                        .zip(alias_upper.iter())
                        .all(|(&a, &b)| a.to_ascii_uppercase() == b);
                    let alias_end = j + alias_len;
                    let alias_after_ok = alias_end >= len || !is_word_char(bytes[alias_end]);
                    if alias_before_ok && matches && alias_after_ok && skip.is_code(j) {
                        return Some(j);
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Finds the byte offset of the first whole-word occurrence of `word` in `source`
/// that is in a code region (not inside strings/comments).
fn find_word_position(source: &str, word: &str) -> Option<usize> {
    let bytes = source.as_bytes();
    let word_upper: Vec<u8> = word.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let wlen = word_upper.len();
    let len = bytes.len();
    let skip = SkipMap::build(source);
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
            if after_ok {
                return Some(i);
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
