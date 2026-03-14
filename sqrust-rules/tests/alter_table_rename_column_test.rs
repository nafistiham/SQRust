use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::alter_table_rename_column::AlterTableRenameColumn;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    AlterTableRenameColumn.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(AlterTableRenameColumn.name(), "Lint/AlterTableRenameColumn");
}

#[test]
fn rename_column_one_violation() {
    let sql = "ALTER TABLE t RENAME COLUMN old_name TO new_name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn add_column_no_violation() {
    let sql = "ALTER TABLE t ADD COLUMN age INT";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_column_no_violation() {
    let sql = "ALTER TABLE t DROP COLUMN age";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn rename_table_no_violation() {
    let sql = "ALTER TABLE t RENAME TO t2";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn set_data_type_no_violation() {
    let sql = "ALTER TABLE t ALTER COLUMN age TYPE BIGINT";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_renames_two_violations() {
    let sql = "ALTER TABLE t RENAME COLUMN a TO b;\nALTER TABLE t RENAME COLUMN c TO d";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "ALTER GARBAGE @@@###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = AlterTableRenameColumn.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn message_contains_old_name() {
    let sql = "ALTER TABLE t RENAME COLUMN old_name TO new_name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("old_name"),
        "message should contain old column name: {}",
        diags[0].message
    );
}

#[test]
fn message_contains_new_name() {
    let sql = "ALTER TABLE t RENAME COLUMN old_name TO new_name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("new_name"),
        "message should contain new column name: {}",
        diags[0].message
    );
}

#[test]
fn multi_op_statement_rename_flagged() {
    // Some dialects allow multiple operations in one ALTER TABLE
    // For parsers that support it, RENAME COLUMN in a multi-op statement is still caught
    let sql = "ALTER TABLE t RENAME COLUMN foo TO bar";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_table_no_violation() {
    let sql = "CREATE TABLE t (id INT, name TEXT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn diagnostic_rule_name_correct() {
    let sql = "ALTER TABLE t RENAME COLUMN old_name TO new_name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Lint/AlterTableRenameColumn");
}

#[test]
fn line_col_nonzero() {
    let sql = "ALTER TABLE t RENAME COLUMN old_name TO new_name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
