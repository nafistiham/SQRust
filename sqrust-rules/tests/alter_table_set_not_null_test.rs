use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::alter_table_set_not_null::AlterTableSetNotNull;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    AlterTableSetNotNull.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(AlterTableSetNotNull.name(), "Lint/AlterTableSetNotNull");
}

#[test]
fn set_not_null_violation() {
    let sql = "ALTER TABLE t ALTER COLUMN c SET NOT NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_not_null_violation() {
    let sql = "ALTER TABLE t ALTER COLUMN c DROP NOT NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn set_not_null_case_insensitive() {
    let sql = "alter table t alter column c set not null";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_not_null_case_insensitive() {
    let sql = "alter table t alter column c drop not null";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn regular_not_null_constraint_no_violation() {
    let sql = "CREATE TABLE t (c INT NOT NULL)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn add_not_null_constraint_no_violation() {
    // ADD CONSTRAINT ... does not contain "SET NOT NULL" or "DROP NOT NULL"
    let sql = "ALTER TABLE t ADD CONSTRAINT chk CHECK (c IS NOT NULL)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn set_not_null_message_content() {
    let sql = "ALTER TABLE t ALTER COLUMN c SET NOT NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let lower = diags[0].message.to_lowercase();
    assert!(
        lower.contains("postgresql") || lower.contains("dialect"),
        "message should mention PostgreSQL or dialect: {}",
        diags[0].message
    );
}

#[test]
fn drop_not_null_message_content() {
    let sql = "ALTER TABLE t ALTER COLUMN c DROP NOT NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let lower = diags[0].message.to_lowercase();
    assert!(
        lower.contains("postgresql") || lower.contains("dialect"),
        "message should mention PostgreSQL or dialect: {}",
        diags[0].message
    );
}

#[test]
fn two_violations_two_detections() {
    let sql = "ALTER TABLE t ALTER COLUMN a SET NOT NULL;\nALTER TABLE t ALTER COLUMN b DROP NOT NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn not_null_in_create_no_violation() {
    let sql = "CREATE TABLE orders (id INT NOT NULL, name VARCHAR(100) NOT NULL)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn line_col_nonzero() {
    let sql = "ALTER TABLE t ALTER COLUMN c SET NOT NULL";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn set_not_null_in_string_no_violation() {
    let sql = "SELECT 'SET NOT NULL example' FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}
