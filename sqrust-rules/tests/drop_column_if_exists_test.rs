use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::drop_column_if_exists::DropColumnIfExists;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DropColumnIfExists.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DropColumnIfExists.name(), "Lint/DropColumnIfExists");
}

#[test]
fn drop_column_without_if_exists_one_violation() {
    let sql = "ALTER TABLE t DROP COLUMN c";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_column_if_exists_no_violation() {
    let sql = "ALTER TABLE t DROP COLUMN IF EXISTS c";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn drop_column_lowercase_violation() {
    let sql = "alter table t drop column c";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_column_if_exists_lowercase_no_violation() {
    let sql = "alter table t drop column if exists c";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn multiple_drop_column_multiple_violations() {
    let sql = "ALTER TABLE t DROP COLUMN a;\nALTER TABLE t DROP COLUMN b";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn mixed_with_and_without_if_exists() {
    let sql = concat!(
        "ALTER TABLE t DROP COLUMN IF EXISTS safe;\n",
        "ALTER TABLE t DROP COLUMN unsafe_col",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_mentions_if_exists() {
    let sql = "ALTER TABLE t DROP COLUMN c";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let lower = diags[0].message.to_lowercase();
    assert!(
        lower.contains("if exists"),
        "message should mention IF EXISTS: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "ALTER TABLE t DROP COLUMN c";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn violation_on_second_line_has_correct_line_number() {
    let sql = concat!(
        "SELECT 1;\n",
        "ALTER TABLE t DROP COLUMN c",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn select_statement_no_violation() {
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
fn skip_drop_column_in_string_literal() {
    let sql = "SELECT 'alter table t drop column c' FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn skip_drop_column_in_line_comment() {
    let sql = "-- alter table t drop column c\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn works_with_parse_error_source_level_scan() {
    // Source-level scan works even with invalid SQL
    let sql = "ALTER TABLE t DROP COLUMN c ### invalid";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn drop_column_mixed_case_without_if_exists_violation() {
    let sql = "Alter Table t Drop Column c";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
