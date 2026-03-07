use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::alter_table_add_not_null_without_default::AlterTableAddNotNullWithoutDefault;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    AlterTableAddNotNullWithoutDefault.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        AlterTableAddNotNullWithoutDefault.name(),
        "Lint/AlterTableAddNotNullWithoutDefault"
    );
}

#[test]
fn create_table_not_flagged() {
    // CREATE TABLE with NOT NULL should not be flagged — only ALTER TABLE
    let sql = "CREATE TABLE t (id INT NOT NULL)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_add_nullable_no_violation() {
    let sql = "ALTER TABLE t ADD COLUMN c VARCHAR(100)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_add_not_null_with_default_no_violation() {
    let sql = "ALTER TABLE t ADD COLUMN c INT NOT NULL DEFAULT 0";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_add_not_null_without_default_one_violation() {
    let sql = "ALTER TABLE t ADD COLUMN c INT NOT NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_insensitive_flagged() {
    let sql = "alter table t add column c int not null";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn in_string_not_flagged() {
    let sql = "SELECT 'ALTER TABLE t ADD COLUMN c INT NOT NULL' FROM x";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn in_comment_not_flagged() {
    let sql = "-- ALTER TABLE t ADD COLUMN c INT NOT NULL\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_statements_both_flagged() {
    let sql = "ALTER TABLE t ADD COLUMN a INT NOT NULL;\nALTER TABLE u ADD COLUMN b TEXT NOT NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn alter_with_default_null_no_violation() {
    // Has DEFAULT keyword (DEFAULT NULL), so the heuristic does not flag
    let sql = "ALTER TABLE t ADD COLUMN c INT DEFAULT NULL";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_not_null() {
    let sql = "ALTER TABLE t ADD COLUMN c INT NOT NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_uppercase().contains("NOT NULL"),
        "message should mention 'NOT NULL', got: {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let sql = "ALTER TABLE t ADD COLUMN c INT NOT NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn col_nonzero() {
    let sql = "ALTER TABLE t ADD COLUMN c INT NOT NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}
