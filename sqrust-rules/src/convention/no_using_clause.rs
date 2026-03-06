use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Join, JoinConstraint, JoinOperator, Query, Select, SetExpr, Statement,
    TableFactor, TableWithJoins};

pub struct NoUsingClause;

/// Converts a byte offset to a 1-indexed (line, col) pair.
fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}

/// Returns `true` if `ch` is a SQL word character (`[a-zA-Z0-9_]`).
#[inline]
fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Builds a skip table: `true` at every byte inside strings, comments, or
/// quoted identifiers.
fn build_skip(bytes: &[u8]) -> Vec<bool> {
    let len = bytes.len();
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Line comment: -- ... newline
        if i + 1 < len && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len && bytes[i] != b'\n' {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Block comment: /* ... */
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            skip[i] = true;
            skip[i + 1] = true;
            i += 2;
            while i < len {
                if i + 1 < len && bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    skip[i] = true;
                    skip[i + 1] = true;
                    i += 2;
                    break;
                }
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Single-quoted string: '...' with '' escape
        if bytes[i] == b'\'' {
            skip[i] = true;
            i += 1;
            while i < len {
                if bytes[i] == b'\'' {
                    skip[i] = true;
                    i += 1;
                    if i < len && bytes[i] == b'\'' {
                        skip[i] = true;
                        i += 1;
                        continue;
                    }
                    break;
                }
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Double-quoted identifier: "..."
        if bytes[i] == b'"' {
            skip[i] = true;
            i += 1;
            while i < len && bytes[i] != b'"' {
                skip[i] = true;
                i += 1;
            }
            if i < len {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        // Backtick identifier: `...`
        if bytes[i] == b'`' {
            skip[i] = true;
            i += 1;
            while i < len && bytes[i] != b'`' {
                skip[i] = true;
                i += 1;
            }
            if i < len {
                skip[i] = true;
                i += 1;
            }
            continue;
        }

        i += 1;
    }

    skip
}

/// Finds the Nth occurrence (0-indexed) of the `USING` keyword (case-insensitive,
/// word-boundary) in `source` outside strings/comments.
///
/// Returns the byte offset of the `U` in `USING`, or `None` if not found.
fn find_nth_using(source: &str, skip: &[bool], occurrence: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let pattern = b"USING";
    let pat_len = pattern.len();
    let mut count = 0;
    let mut i = 0;

    while i + pat_len <= len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Case-insensitive match
        let matches = bytes[i..i + pat_len]
            .iter()
            .zip(pattern.iter())
            .all(|(&a, &b)| a.eq_ignore_ascii_case(&b));

        if matches {
            let all_code = (i..i + pat_len).all(|k| !skip[k]);
            if all_code {
                let boundary_before = i == 0 || !is_word_char(bytes[i - 1]);
                let end = i + pat_len;
                let boundary_after = end >= len || !is_word_char(bytes[end]);
                if boundary_before && boundary_after {
                    if count == occurrence {
                        return Some(i);
                    }
                    count += 1;
                    i += pat_len;
                    continue;
                }
            }
        }

        i += 1;
    }

    None
}

/// Returns `true` if the join has a `USING(...)` constraint.
fn join_has_using(join: &Join) -> bool {
    let constraint = match &join.join_operator {
        JoinOperator::Inner(c) => Some(c),
        JoinOperator::LeftOuter(c) => Some(c),
        JoinOperator::RightOuter(c) => Some(c),
        JoinOperator::FullOuter(c) => Some(c),
        JoinOperator::Semi(c) => Some(c),
        JoinOperator::LeftSemi(c) => Some(c),
        JoinOperator::RightSemi(c) => Some(c),
        JoinOperator::Anti(c) => Some(c),
        JoinOperator::LeftAnti(c) => Some(c),
        JoinOperator::RightAnti(c) => Some(c),
        JoinOperator::CrossJoin
        | JoinOperator::CrossApply
        | JoinOperator::OuterApply
        | JoinOperator::AsOf { .. } => None,
    };
    matches!(constraint, Some(JoinConstraint::Using(_)))
}

/// Recurses into a `TableFactor` to find derived-table subqueries.
fn collect_from_table_factor(
    factor: &TableFactor,
    source: &str,
    skip: &[bool],
    using_count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = factor {
        collect_from_query(subquery, source, skip, using_count, diags);
    }
}

/// Collects USING violations from a list of `TableWithJoins` items.
/// `using_count` tracks how many USING occurrences in the source we've consumed,
/// so each violation maps to the correct text position.
///
/// For each `TableWithJoins` we first recurse into the relation (which may be
/// a derived subquery), then into each join's relation, and then check whether
/// the join itself uses `USING`.
fn collect_from_table_with_joins(
    tables: &[TableWithJoins],
    source: &str,
    skip: &[bool],
    using_count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    for twj in tables {
        // Recurse into the primary relation (e.g. a derived subquery).
        collect_from_table_factor(&twj.relation, source, skip, using_count, diags);

        for join in &twj.joins {
            // Recurse into the join's relation (may itself be a subquery).
            collect_from_table_factor(&join.relation, source, skip, using_count, diags);

            if join_has_using(join) {
                if let Some(offset) = find_nth_using(source, skip, *using_count) {
                    let (line, col) = line_col(source, offset);
                    diags.push(Diagnostic {
                        rule: "Convention/NoUsingClause",
                        message:
                            "JOIN USING clause found; prefer explicit ON conditions for clarity"
                                .to_string(),
                        line,
                        col,
                    });
                }
                *using_count += 1;
            }
        }
    }
}

fn collect_from_select(
    select: &Select,
    source: &str,
    skip: &[bool],
    using_count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    collect_from_table_with_joins(&select.from, source, skip, using_count, diags);
}

fn collect_from_set_expr(
    expr: &SetExpr,
    source: &str,
    skip: &[bool],
    using_count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(select) => collect_from_select(select, source, skip, using_count, diags),
        SetExpr::Query(inner) => collect_from_query(inner, source, skip, using_count, diags),
        SetExpr::SetOperation { left, right, .. } => {
            collect_from_set_expr(left, source, skip, using_count, diags);
            collect_from_set_expr(right, source, skip, using_count, diags);
        }
        _ => {}
    }
}

fn collect_from_query(
    query: &Query,
    source: &str,
    skip: &[bool],
    using_count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // CTEs
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            collect_from_query(&cte.query, source, skip, using_count, diags);
        }
    }
    collect_from_set_expr(&query.body, source, skip, using_count, diags);
}

impl Rule for NoUsingClause {
    fn name(&self) -> &'static str {
        "Convention/NoUsingClause"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let source = &ctx.source;
        let bytes = source.as_bytes();
        let skip = build_skip(bytes);
        let mut diags = Vec::new();
        let mut using_count = 0usize;

        for stmt in &ctx.statements {
            if let Statement::Query(query) = stmt {
                collect_from_query(query, source, &skip, &mut using_count, &mut diags);
            }
        }

        diags
    }
}
