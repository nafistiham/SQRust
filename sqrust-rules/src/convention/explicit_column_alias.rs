use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, Select, SelectItem, SetExpr, Statement, TableFactor};

pub struct ExplicitColumnAlias;

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Returns true if `b` is a valid identifier/word character.
fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Given source text and an alias name, find the Nth (0-indexed) occurrence of
/// the alias as a standalone word. Returns the byte offset of that word, or None.
fn find_alias_occurrence(source: &str, alias: &str, occurrence: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let alias_bytes = alias.as_bytes();
    let alias_len = alias_bytes.len();
    let src_len = bytes.len();
    let mut count = 0usize;
    let mut i = 0;

    while i + alias_len <= src_len {
        // Word boundary before
        let before_ok = i == 0 || !is_word(bytes[i - 1]);
        if before_ok {
            let matched = bytes[i..i + alias_len]
                .iter()
                .zip(alias_bytes.iter())
                .all(|(&a, b)| a.eq_ignore_ascii_case(b));
            if matched {
                // Word boundary after
                let after_ok = i + alias_len >= src_len || !is_word(bytes[i + alias_len]);
                if after_ok {
                    if count == occurrence {
                        return Some(i);
                    }
                    count += 1;
                }
            }
        }
        i += 1;
    }
    None
}

/// Check if `AS` (case-insensitive) appears immediately before `pos` in source,
/// allowing arbitrary whitespace between `AS` and `pos`.
/// Also handles quoted aliases (pos might point into a quoted string).
fn has_as_before(source: &str, pos: usize) -> bool {
    let bytes = source.as_bytes();

    // Walk backwards from pos, skipping whitespace.
    let mut j = pos;
    if j == 0 {
        return false;
    }
    // Also skip a leading quote character if the alias is quoted.
    if j > 0 && (bytes[j - 1] == b'"' || bytes[j - 1] == b'`' || bytes[j - 1] == b'\'') {
        // The alias starts after a quote — back up past the quote too.
        j -= 1;
    }

    // Now skip whitespace going backward.
    while j > 0 && (bytes[j - 1] == b' ' || bytes[j - 1] == b'\t' || bytes[j - 1] == b'\n' || bytes[j - 1] == b'\r') {
        j -= 1;
    }

    if j < 2 {
        return false;
    }

    // Check if the two characters before are 'AS' (case-insensitive),
    // and that the character before that is not a word char (word boundary).
    let candidate = &bytes[j - 2..j];
    if !candidate.eq_ignore_ascii_case(b"AS") {
        return false;
    }
    // Word boundary before 'AS'
    let before_as = j - 2;
    if before_as > 0 && is_word(bytes[before_as - 1]) {
        return false;
    }
    true
}

/// Collect (alias_name, occurrence_index) pairs per alias name from a SELECT list,
/// then check each one in source text.
fn check_projection(
    projection: &[SelectItem],
    source: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    alias_counts: &mut std::collections::HashMap<String, usize>,
) {
    for item in projection {
        if let SelectItem::ExprWithAlias { alias, .. } = item {
            let alias_str = alias.value.as_str();
            let key = alias_str.to_lowercase();
            let occ = *alias_counts.get(&key).unwrap_or(&0);
            *alias_counts.entry(key).or_insert(0) += 1;

            if let Some(pos) = find_alias_occurrence(source, alias_str, occ) {
                if !has_as_before(source, pos) {
                    let (line, col) = line_col(source, pos);
                    diags.push(Diagnostic {
                        rule,
                        message: format!(
                            "Column alias '{}' omits the AS keyword — use 'expression AS alias' for clarity",
                            alias_str
                        ),
                        line,
                        col,
                    });
                }
            }
        }
    }
}

fn check_select(
    sel: &Select,
    source: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    alias_counts: &mut std::collections::HashMap<String, usize>,
) {
    check_projection(&sel.projection, source, rule, diags, alias_counts);

    // Recurse into subqueries in FROM.
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, source, rule, diags, alias_counts);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, source, rule, diags, alias_counts);
        }
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    source: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    alias_counts: &mut std::collections::HashMap<String, usize>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, rule, diags, alias_counts);
    }
}

fn check_set_expr(
    expr: &SetExpr,
    source: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    alias_counts: &mut std::collections::HashMap<String, usize>,
) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, source, rule, diags, alias_counts),
        SetExpr::Query(inner) => check_query(inner, source, rule, diags, alias_counts),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, rule, diags, alias_counts);
            check_set_expr(right, source, rule, diags, alias_counts);
        }
        _ => {}
    }
}

fn check_query(
    query: &Query,
    source: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
    alias_counts: &mut std::collections::HashMap<String, usize>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, rule, diags, alias_counts);
        }
    }
    check_set_expr(&query.body, source, rule, diags, alias_counts);
}

impl Rule for ExplicitColumnAlias {
    fn name(&self) -> &'static str {
        "Convention/ExplicitColumnAlias"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();
        // Track how many times each alias name has been seen (case-insensitive key)
        // so we can find the correct Nth occurrence in source text.
        let mut alias_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                check_query(
                    query,
                    &ctx.source,
                    self.name(),
                    &mut diags,
                    &mut alias_counts,
                );
            }
        }

        diags
    }
}
