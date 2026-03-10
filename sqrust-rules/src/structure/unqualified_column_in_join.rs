use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, FunctionArgExpr, GroupByExpr, Query, Select, SelectItem, SetExpr,
    Statement, TableFactor, With,
};

pub struct UnqualifiedColumnInJoin;

impl Rule for UnqualifiedColumnInJoin {
    fn name(&self) -> &'static str {
        "Structure/UnqualifiedColumnInJoin"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, &ctx.source, self.name(), &mut diags);
            }
        }
        diags
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
    // Recurse into subqueries in FROM clause regardless of join presence
    for twj in &sel.from {
        recurse_table_factor(&twj.relation, src, rule, diags);
        for join in &twj.joins {
            recurse_table_factor(&join.relation, src, rule, diags);
        }
    }

    // Only flag unqualified columns when there are explicit JOINs
    let has_joins = sel.from.iter().any(|twj| !twj.joins.is_empty());
    if !has_joins {
        return;
    }

    // Check SELECT projections
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                find_unqualified(e, src, rule, diags);
            }
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => {}
        }
    }

    // Check WHERE
    if let Some(w) = &sel.selection {
        find_unqualified(w, src, rule, diags);
    }

    // Check HAVING
    if let Some(h) = &sel.having {
        find_unqualified(h, src, rule, diags);
    }

    // Check GROUP BY
    if let GroupByExpr::Expressions(exprs, _) = &sel.group_by {
        for g in exprs {
            find_unqualified(g, src, rule, diags);
        }
    }
}

fn recurse_table_factor(tf: &TableFactor, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, src, rule, diags);
    }
}

fn find_unqualified(expr: &Expr, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Identifier(i) => {
            // Unqualified column reference — flag it
            if let Some(off) = find_word_in_source(src, &i.value, 0) {
                let (line, col) = offset_to_line_col(src, off);
                diags.push(Diagnostic {
                    rule,
                    message: format!(
                        "Column '{}' is not qualified with a table name or alias; in a JOIN query, all columns should be table-qualified",
                        i.value
                    ),
                    line,
                    col,
                });
            }
        }
        Expr::CompoundIdentifier(_) => {} // Qualified — ok
        Expr::BinaryOp { left, right, .. } => {
            find_unqualified(left, src, rule, diags);
            find_unqualified(right, src, rule, diags);
        }
        Expr::UnaryOp { expr, .. } | Expr::Nested(expr) => {
            find_unqualified(expr, src, rule, diags);
        }
        Expr::Function(f) => {
            if let sqlparser::ast::FunctionArguments::List(arg_list) = &f.args {
                for arg in &arg_list.args {
                    if let sqlparser::ast::FunctionArg::Unnamed(arg_expr) = arg {
                        if let FunctionArgExpr::Expr(e) = arg_expr {
                            find_unqualified(e, src, rule, diags);
                        }
                    }
                }
            }
        }
        Expr::IsNull(e) | Expr::IsNotNull(e) => find_unqualified(e, src, rule, diags),
        Expr::Between { expr, low, high, .. } => {
            find_unqualified(expr, src, rule, diags);
            find_unqualified(low, src, rule, diags);
            find_unqualified(high, src, rule, diags);
        }
        Expr::InList { expr, list, .. } => {
            find_unqualified(expr, src, rule, diags);
            for e in list {
                find_unqualified(e, src, rule, diags);
            }
        }
        Expr::Case { operand, conditions, results, else_result } => {
            if let Some(e) = operand {
                find_unqualified(e, src, rule, diags);
            }
            for (c, r) in conditions.iter().zip(results.iter()) {
                find_unqualified(c, src, rule, diags);
                find_unqualified(r, src, rule, diags);
            }
            if let Some(e) = else_result {
                find_unqualified(e, src, rule, diags);
            }
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
