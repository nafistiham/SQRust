use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::add_column_without_default::AddColumnWithoutDefault;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    AddColumnWithoutDefault.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        AddColumnWithoutDefault.name(),
        "Lint/AddColumnWithoutDefault"
    );
}

#[test]
fn add_column_without_default_one_violation() {
    let sql = "ALTER TABLE t ADD COLUMN c VARCHAR(100)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn add_column_with_default_no_violation() {
    let sql = "ALTER TABLE t ADD COLUMN c VARCHAR(100) DEFAULT ''";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn add_column_with_default_null_no_violation() {
    let sql = "ALTER TABLE t ADD COLUMN c INT DEFAULT NULL";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn add_column_with_default_zero_no_violation() {
    let sql = "ALTER TABLE t ADD COLUMN c INT DEFAULT 0";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn case_insensitive_flagged() {
    let sql = "alter table t add column c int";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_case_flagged() {
    let sql = "Alter Table t Add Column c Text";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_string_literal_not_flagged() {
    let sql = "SELECT 'ALTER TABLE t ADD COLUMN c INT' FROM x";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn in_line_comment_not_flagged() {
    let sql = "-- ALTER TABLE t ADD COLUMN c INT\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn in_block_comment_not_flagged() {
    let sql = "/* ALTER TABLE t ADD COLUMN c INT */\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_statements_both_without_default_two_violations() {
    let sql =
        "ALTER TABLE t ADD COLUMN a INT;\nALTER TABLE u ADD COLUMN b TEXT";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn two_statements_one_with_default_one_violation() {
    let sql =
        "ALTER TABLE t ADD COLUMN a INT DEFAULT 0;\nALTER TABLE u ADD COLUMN b TEXT";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_mentions_default() {
    let sql = "ALTER TABLE t ADD COLUMN c INT";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_uppercase();
    assert!(
        msg.contains("DEFAULT"),
        "message should mention DEFAULT, got: {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let sql = "ALTER TABLE t ADD COLUMN c INT";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn col_nonzero() {
    let sql = "ALTER TABLE t ADD COLUMN c INT";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn line_col_second_line() {
    let sql = "SELECT 1;\nALTER TABLE t ADD COLUMN c INT";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn not_null_with_default_no_violation() {
    // NOT NULL + DEFAULT is acceptable
    let sql = "ALTER TABLE t ADD COLUMN c INT NOT NULL DEFAULT 0";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_table_not_flagged() {
    // CREATE TABLE with column definitions should not be flagged
    let sql = "CREATE TABLE t (c INT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}
