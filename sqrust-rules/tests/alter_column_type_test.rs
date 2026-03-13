use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::alter_column_type::AlterColumnType;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    AlterColumnType.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(AlterColumnType.name(), "Lint/AlterColumnType");
}

#[test]
fn alter_column_type_postgres_syntax_parse_error() {
    // PostgreSQL-only `ALTER COLUMN c TYPE <new_type>` (no SET DATA) does not parse
    // under GenericDialect — the project uses GenericDialect, so this is a parse error
    // and the rule correctly returns 0 violations (handled by parse_errors guard).
    let sql = "ALTER TABLE t ALTER COLUMN c TYPE VARCHAR(100)";
    let ctx = sqrust_core::FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        // Expected: parse error → 0 violations
        let diags = AlterColumnType.check(&ctx);
        assert!(diags.is_empty());
    } else {
        // If somehow it parses (future dialect support), there should be 1 violation
        let diags = AlterColumnType.check(&ctx);
        assert_eq!(diags.len(), 1);
    }
}

#[test]
fn alter_column_set_data_type_one_violation() {
    // Standard: ALTER COLUMN c SET DATA TYPE INT
    let sql = "ALTER TABLE t ALTER COLUMN c SET DATA TYPE INT";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn alter_table_add_column_no_violation() {
    let sql = "ALTER TABLE t ADD COLUMN c VARCHAR(100)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_table_drop_column_no_violation() {
    let sql = "ALTER TABLE t DROP COLUMN c";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_table_rename_column_no_violation() {
    let sql = "ALTER TABLE t RENAME COLUMN a TO b";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_column_set_not_null_no_violation() {
    let sql = "ALTER TABLE t ALTER COLUMN c SET NOT NULL";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_column_drop_not_null_no_violation() {
    let sql = "ALTER TABLE t ALTER COLUMN c DROP NOT NULL";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_column_set_default_no_violation() {
    let sql = "ALTER TABLE t ALTER COLUMN c SET DEFAULT 0";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_alter_column_type_multiple_violations() {
    let sql = "ALTER TABLE t ALTER COLUMN a SET DATA TYPE INT;\nALTER TABLE t ALTER COLUMN b SET DATA TYPE TEXT";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "ALTER GARBAGE @@@###";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = AlterColumnType.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn create_table_no_violation() {
    let sql = "CREATE TABLE t (c INT)";
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
fn message_contains_data_type() {
    let sql = "ALTER TABLE t ALTER COLUMN c SET DATA TYPE INT";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("data type") || msg.contains("type"),
        "expected message to contain 'data type' or 'type', got: {}",
        msg
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "ALTER TABLE t ALTER COLUMN c SET DATA TYPE INT";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
