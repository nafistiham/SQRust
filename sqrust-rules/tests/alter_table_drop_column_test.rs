use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::alter_table_drop_column::AlterTableDropColumn;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    AlterTableDropColumn.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(AlterTableDropColumn.name(), "Lint/AlterTableDropColumn");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "ALTER GARBAGE @@@###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = AlterTableDropColumn.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn alter_table_drop_column_one_violation() {
    let sql = "ALTER TABLE t DROP COLUMN name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn alter_table_add_column_no_violation() {
    let sql = "ALTER TABLE t ADD COLUMN name TEXT";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_table_rename_no_violation() {
    let sql = "ALTER TABLE t RENAME TO new_name";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_no_violation() {
    let sql = "SELECT id, name FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_table_no_violation() {
    let sql = "DROP TABLE t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_drop_columns_two_violations() {
    let sql = "ALTER TABLE t DROP COLUMN a;\nALTER TABLE t DROP COLUMN b";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn drop_column_if_exists_still_violation() {
    let sql = "ALTER TABLE t DROP COLUMN IF EXISTS name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_useful_text() {
    let sql = "ALTER TABLE t DROP COLUMN name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("irreversible"),
        "expected 'irreversible' in message, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "ALTER TABLE t DROP COLUMN name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn alter_table_modify_column_no_violation() {
    let sql = "ALTER TABLE t ALTER COLUMN name TYPE INT";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn update_no_violation() {
    let sql = "UPDATE t SET name = 'foo' WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn correct_line_for_alter_keyword() {
    // ALTER is on line 2
    let sql = "SELECT 1;\nALTER TABLE t DROP COLUMN name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}
