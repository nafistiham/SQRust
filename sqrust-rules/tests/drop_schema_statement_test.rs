use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::drop_schema_statement::DropSchemaStatement;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DropSchemaStatement.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DropSchemaStatement.name(), "Lint/DropSchemaStatement");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "NOT VALID SQL ###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = DropSchemaStatement.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn drop_schema_one_violation() {
    let sql = "DROP SCHEMA my_schema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_database_one_violation() {
    let sql = "DROP DATABASE my_db";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_schema_if_exists_still_violation() {
    // IF EXISTS does not make DROP SCHEMA any safer — still flagged
    let sql = "DROP SCHEMA IF EXISTS my_schema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_drop_schemas_two_violations() {
    let sql = "DROP SCHEMA s1;\nDROP SCHEMA s2";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn drop_table_no_violation() {
    // DROP TABLE is a different rule — should not be flagged here
    let sql = "DROP TABLE t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_no_violation() {
    let sql = "SELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_schema_no_violation() {
    let sql = "CREATE SCHEMA my_schema";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_contains_irreversible_or_backup() {
    let sql = "DROP SCHEMA my_schema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("irreversible") || msg.contains("backup"),
        "message should mention irreversible or backup: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "DROP SCHEMA my_schema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn correct_line_for_drop_keyword() {
    let sql = "SELECT 1;\nDROP SCHEMA my_schema";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn drop_schema_cascade_violation() {
    // CASCADE does not exempt DROP SCHEMA from being flagged
    let sql = "DROP SCHEMA my_schema CASCADE";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_database_if_exists_violation() {
    let sql = "DROP DATABASE IF EXISTS my_db";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
