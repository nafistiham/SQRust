use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::drop_table_if_exists::DropTableIfExists;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DropTableIfExists.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DropTableIfExists.name(), "Lint/DropTableIfExists");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "NOT VALID SQL ###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = DropTableIfExists.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn drop_table_without_if_exists_one_violation() {
    let sql = "DROP TABLE t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_table_if_exists_no_violation() {
    let sql = "DROP TABLE IF EXISTS t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_table_multiple_tables_one_violation() {
    // DROP TABLE a, b — sqlparser parses this as one Drop statement with names=[a, b]
    let sql = "DROP TABLE a, b";
    let diags = check(sql);
    // One statement — one violation (the statement lacks IF EXISTS)
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_statement_no_violation() {
    let sql = "SELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn delete_without_where_no_violation() {
    // Different rule — DROP TABLE rule must not flag DELETE
    let sql = "DELETE FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_contains_useful_text() {
    let sql = "DROP TABLE t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("if exists") || msg.contains("exist"),
        "message should mention IF EXISTS: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "DROP TABLE t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn multiple_drop_statements_multiple_violations() {
    let sql = "DROP TABLE a;\nDROP TABLE b";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn drop_view_no_violation() {
    // Rule only flags DROP TABLE, not DROP VIEW
    let sql = "DROP VIEW v";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_index_no_violation() {
    // Rule only flags DROP TABLE, not DROP INDEX
    let sql = "DROP INDEX idx";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn update_statement_no_violation() {
    let sql = "UPDATE t SET a = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_table_lowercase_without_if_exists_one_violation() {
    let sql = "drop table t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_table_if_exists_lowercase_no_violation() {
    let sql = "drop table if exists t";
    let diags = check(sql);
    assert!(diags.is_empty());
}
