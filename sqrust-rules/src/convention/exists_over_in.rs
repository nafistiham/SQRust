use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, Join, JoinConstraint, JoinOperator, Query, Select, SelectItem, SetExpr, Statement,
    TableFactor,
};

pub struct ExistsOverIn;

impl Rule for ExistsOverIn {
    fn name(&self) -> &'static str {
        "Convention/ExistsOverIn"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        // Pre-collect all `IN (SELECT ...)` byte offsets in source order so we
        // can assign accurate line/col to each AST-detected violation.
        let offsets = collect_in_subquery_offsets(&ctx.source);
        let mut offset_idx: usize = 0;
        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            check_statement(stmt, &ctx.source, &offsets, &mut offset_idx, &mut diags);
        }

        diags
    }
}

// ── statement walker ──────────────────────────────────────────────────────────

fn check_statement(
    stmt: &Statement,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match stmt {
        Statement::Query(q) => check_query(q, source, offsets, idx, diags),
        Statement::Insert(insert) => {
            if let Some(src) = &insert.source {
                check_query(src, source, offsets, idx, diags);
            }
        }
        Statement::Update { selection, .. } => {
            if let Some(expr) = selection {
                check_expr(expr, source, offsets, idx, diags);
            }
        }
        Statement::Delete(delete) => {
            if let Some(expr) = &delete.selection {
                check_expr(expr, source, offsets, idx, diags);
            }
        }
        _ => {}
    }
}

// ── query / set-expr walker ───────────────────────────────────────────────────

fn check_query(
    query: &Query,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &query.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, source, offsets, idx, diags);
        }
    }
    check_set_expr(&query.body, source, offsets, idx, diags);
}

fn check_set_expr(
    body: &SetExpr,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match body {
        SetExpr::Select(sel) => check_select(sel, source, offsets, idx, diags),
        SetExpr::Query(q) => check_query(q, source, offsets, idx, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, source, offsets, idx, diags);
            check_set_expr(right, source, offsets, idx, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // Recurse into table factors (subqueries in FROM) and JOIN ON conditions.
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, source, offsets, idx, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, source, offsets, idx, diags);
            check_join_condition(join, source, offsets, idx, diags);
        }
    }

    // Projection
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                check_expr(e, source, offsets, idx, diags);
            }
            _ => {}
        }
    }

    // WHERE
    if let Some(selection) = &sel.selection {
        check_expr(selection, source, offsets, idx, diags);
    }

    // HAVING
    if let Some(having) = &sel.having {
        check_expr(having, source, offsets, idx, diags);
    }
}

fn check_join_condition(
    join: &Join,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    let on_expr = match &join.join_operator {
        JoinOperator::Inner(JoinConstraint::On(e))
        | JoinOperator::LeftOuter(JoinConstraint::On(e))
        | JoinOperator::RightOuter(JoinConstraint::On(e))
        | JoinOperator::FullOuter(JoinConstraint::On(e)) => Some(e),
        _ => None,
    };
    if let Some(expr) = on_expr {
        check_expr(expr, source, offsets, idx, diags);
    }
}

fn recurse_table_factor(
    tf: &TableFactor,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, source, offsets, idx, diags);
    }
}

// ── expression walker ─────────────────────────────────────────────────────────

fn check_expr(
    expr: &Expr,
    source: &str,
    offsets: &[usize],
    idx: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        // `col IN (SELECT ...)` — the target pattern.
        Expr::InSubquery {
            expr: inner,
            subquery,
            negated,
        } => {
            check_expr(inner, source, offsets, idx, diags);

            if !negated {
                // Consume the next pre-collected offset to get accurate line/col.
                let offset = offsets.get(*idx).copied().unwrap_or(0);
                let (line, col) = offset_to_line_col(source, offset);
                diags.push(Diagnostic {
                    rule: "Convention/ExistsOverIn",
                    message: "Use EXISTS instead of IN with a subquery for better performance"
                        .to_string(),
                    line,
                    col,
                });
                *idx += 1;
            }
            // NOT IN (SELECT ...) — not flagged; don't consume an offset because
            // `collect_in_subquery_offsets` only records plain `IN (SELECT`.

            check_query(subquery, source, offsets, idx, diags);
        }

        // `col IN (1, 2, 3)` — literal list; not flagged, just recurse.
        Expr::InList {
            expr: inner,
            list,
            ..
        } => {
            check_expr(inner, source, offsets, idx, diags);
            for e in list {
                check_expr(e, source, offsets, idx, diags);
            }
        }

        Expr::BinaryOp { left, right, .. } => {
            check_expr(left, source, offsets, idx, diags);
            check_expr(right, source, offsets, idx, diags);
        }

        Expr::UnaryOp { expr: inner, .. } => {
            check_expr(inner, source, offsets, idx, diags);
        }

        Expr::Nested(inner) => {
            check_expr(inner, source, offsets, idx, diags);
        }

        Expr::Subquery(q) => {
            check_query(q, source, offsets, idx, diags);
        }

        Expr::Exists { subquery, .. } => {
            check_query(subquery, source, offsets, idx, diags);
        }

        Expr::Function(f) => {
            use sqlparser::ast::{FunctionArg, FunctionArgExpr, FunctionArguments};
            if let FunctionArguments::List(arg_list) = &f.args {
                for arg in &arg_list.args {
                    if let FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) = arg {
                        check_expr(e, source, offsets, idx, diags);
                    }
                }
            }
        }

        Expr::Case {
            operand,
            conditions,
            results,
            else_result,
        } => {
            if let Some(op) = operand {
                check_expr(op, source, offsets, idx, diags);
            }
            for cond in conditions {
                check_expr(cond, source, offsets, idx, diags);
            }
            for res in results {
                check_expr(res, source, offsets, idx, diags);
            }
            if let Some(el) = else_result {
                check_expr(el, source, offsets, idx, diags);
            }
        }

        Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
            check_expr(inner, source, offsets, idx, diags);
        }

        Expr::Between {
            expr: inner,
            low,
            high,
            ..
        } => {
            check_expr(inner, source, offsets, idx, diags);
            check_expr(low, source, offsets, idx, diags);
            check_expr(high, source, offsets, idx, diags);
        }

        Expr::Like {
            expr: inner,
            pattern,
            ..
        }
        | Expr::ILike {
            expr: inner,
            pattern,
            ..
        } => {
            check_expr(inner, source, offsets, idx, diags);
            check_expr(pattern, source, offsets, idx, diags);
        }

        _ => {}
    }
}

// ── source-text helpers ───────────────────────────────────────────────────────

/// Collects byte offsets of every `IN` keyword (word-boundary, case-insensitive,
/// outside strings/comments) that is followed (after optional whitespace) by
/// `(` and then optionally whitespace and then `SELECT` or `WITH` (word-boundary).
///
/// These are the `IN (SELECT ...)` / `IN (WITH cte AS (...) SELECT ...)` patterns.
fn collect_in_subquery_offsets(source: &str) -> Vec<usize> {
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut offsets = Vec::new();
    let mut i = 0;

    let skip = build_skip(bytes);

    while i + 2 <= len {
        if skip[i] {
            i += 1;
            continue;
        }

        // Word boundary before `IN`.
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok {
            i += 1;
            continue;
        }

        // Case-insensitive `IN`.
        if !(bytes[i] == b'I' || bytes[i] == b'i') {
            i += 1;
            continue;
        }
        if i + 1 >= len || !(bytes[i + 1] == b'N' || bytes[i + 1] == b'n') {
            i += 1;
            continue;
        }

        // Word boundary after `IN`.
        let after_in = i + 2;
        let after_ok = after_in >= len || !is_word_char(bytes[after_in]);
        if !after_ok {
            i += 1;
            continue;
        }

        // Both bytes of `IN` must be code (not in string/comment).
        if !skip[i] && (i + 1 >= len || !skip[i + 1]) {
            // Scan forward past whitespace to find `(`.
            let mut j = after_in;
            while j < len
                && (bytes[j] == b' '
                    || bytes[j] == b'\t'
                    || bytes[j] == b'\n'
                    || bytes[j] == b'\r')
            {
                j += 1;
            }
            if j < len && bytes[j] == b'(' {
                // Scan past `(` and whitespace to check for SELECT or WITH.
                let mut k = j + 1;
                while k < len
                    && (bytes[k] == b' '
                        || bytes[k] == b'\t'
                        || bytes[k] == b'\n'
                        || bytes[k] == b'\r')
                {
                    k += 1;
                }
                // Check for SELECT (6 chars).
                if k + 6 <= len {
                    let candidate = &bytes[k..k + 6];
                    let is_select = candidate.eq_ignore_ascii_case(b"SELECT");
                    let sel_after = k + 6;
                    let sel_after_ok = sel_after >= len || !is_word_char(bytes[sel_after]);
                    if is_select && sel_after_ok {
                        offsets.push(i);
                        i += 1;
                        continue;
                    }
                }
                // Check for WITH (4 chars) — CTE-based subquery.
                if k + 4 <= len {
                    let candidate = &bytes[k..k + 4];
                    let is_with = candidate.eq_ignore_ascii_case(b"WITH");
                    let with_after = k + 4;
                    let with_after_ok = with_after >= len || !is_word_char(bytes[with_after]);
                    if is_with && with_after_ok {
                        offsets.push(i);
                        i += 1;
                        continue;
                    }
                }
            }
        }

        i += 1;
    }

    offsets
}

/// Builds a boolean skip table: `true` at each byte position that is inside a
/// string literal or comment (and therefore invisible to keyword scanning).
fn build_skip(bytes: &[u8]) -> Vec<bool> {
    let len = bytes.len();
    let mut skip = vec![false; len];
    let mut i = 0;

    while i < len {
        // Line comment: `-- ...`
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
        // Block comment: `/* ... */`
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
        // Single-quoted string: `'...'` with `''` escape.
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
        // Double-quoted identifier: `"..."`.
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
        i += 1;
    }

    skip
}

fn is_word_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let safe_offset = offset.min(source.len());
    let before = &source[..safe_offset];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before
        .rfind('\n')
        .map(|p| safe_offset - p - 1)
        .unwrap_or(safe_offset)
        + 1;
    (line, col)
}
