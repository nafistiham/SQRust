use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Delete, FromTable, Query, Select, SetExpr, Statement, TableFactor, TableWithJoins,
};

pub struct CrossDatabaseReference;

impl Rule for CrossDatabaseReference {
    fn name(&self) -> &'static str {
        "Lint/CrossDatabaseReference"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // Skip files that failed to parse — AST may be incomplete.
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }

        let mut diags = Vec::new();

        for stmt in &ctx.statements {
            match stmt {
                Statement::Query(q) => {
                    check_query(q, ctx, &mut diags);
                }
                Statement::Insert(insert) => {
                    // Check the target table of INSERT INTO
                    if insert.table_name.0.len() >= 3 {
                        let name_str = insert
                            .table_name
                            .0
                            .iter()
                            .map(|i| i.value.as_str())
                            .collect::<Vec<_>>()
                            .join(".");
                        let (line, col) =
                            find_name_position(ctx.source.as_str(), &name_str);
                        diags.push(make_diagnostic(ctx, name_str, line, col));
                    }
                }
                Statement::Update {
                    table: TableWithJoins { relation, joins },
                    ..
                } => {
                    check_table_factor(relation, ctx, &mut diags);
                    for join in joins {
                        check_table_factor(&join.relation, ctx, &mut diags);
                    }
                }
                Statement::Delete(Delete { from, .. }) => {
                    let tables = match from {
                        FromTable::WithFromKeyword(v) | FromTable::WithoutKeyword(v) => v,
                    };
                    for twj in tables {
                        check_table_with_joins(twj, ctx, &mut diags);
                    }
                }
                _ => {}
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
        check_table_with_joins(twj, ctx, diags);
    }
}

fn check_table_with_joins(
    twj: &TableWithJoins,
    ctx: &FileContext,
    diags: &mut Vec<Diagnostic>,
) {
    check_table_factor(&twj.relation, ctx, diags);
    for join in &twj.joins {
        check_table_factor(&join.relation, ctx, diags);
    }
}

fn check_table_factor(tf: &TableFactor, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Table { name, .. } = tf {
        if name.0.len() >= 3 {
            let name_str = name
                .0
                .iter()
                .map(|i| i.value.as_str())
                .collect::<Vec<_>>()
                .join(".");
            let (line, col) = find_name_position(ctx.source.as_str(), &name_str);
            diags.push(make_diagnostic(ctx, name_str, line, col));
        }
    }
}

fn make_diagnostic(
    _ctx: &FileContext,
    name_str: String,
    line: usize,
    col: usize,
) -> Diagnostic {
    Diagnostic {
        rule: "Lint/CrossDatabaseReference",
        message: format!(
            "Cross-database table reference '{}' — in dbt, use ref() or source() macros \
             instead of hardcoded cross-database paths",
            name_str
        ),
        line,
        col,
    }
}

/// Finds the 1-indexed (line, col) of the first occurrence of `name` in `source`.
/// Falls back to (1, 1) if not found.
fn find_name_position(source: &str, name: &str) -> (usize, usize) {
    let source_upper = source.to_uppercase();
    let name_upper = name.to_uppercase();
    if let Some(pos) = source_upper.find(&name_upper) {
        return offset_to_line_col(source, pos);
    }
    (1, 1)
}

/// Converts a byte offset in `source` to a 1-indexed (line, col) pair.
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
