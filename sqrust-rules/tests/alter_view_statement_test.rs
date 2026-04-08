use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::alter_view_statement::AlterViewStatement;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    AlterViewStatement.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(AlterViewStatement.name(), "Lint/AlterViewStatement");
}

#[test]
fn basic_alter_view_violation() {
    let sql = "ALTER VIEW my_view AS\nSELECT id, name FROM users WHERE active = 1;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn single_line_query_no_violation() {
    let sql = "SELECT * FROM my_view WHERE status = 'active'";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_view_in_string_no_violation() {
    let sql = "SELECT 'ALTER VIEW foo' AS example FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_view_in_comment_no_violation() {
    let sql = "-- ALTER VIEW my_view AS SELECT 1\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_view_in_block_comment_no_violation() {
    let sql = "/* ALTER VIEW my_view AS SELECT 1 */\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn alter_view_line_and_col() {
    let sql = "ALTER VIEW report_view AS SELECT a, b FROM source;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn alter_view_after_select() {
    let sql = "SELECT 1;\nALTER VIEW v AS SELECT 2;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn empty_file_no_violation() {
    let sql = "";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn uppercase_violation() {
    let sql = "ALTER VIEW my_view AS SELECT id FROM t;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_violation() {
    let sql = "alter view my_view as select id from t;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_case_violation() {
    let sql = "Alter View my_view As Select id From t;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn alter_table_no_violation() {
    let sql = "ALTER TABLE orders ADD COLUMN status TEXT";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_alter_view() {
    let sql = "ALTER VIEW v AS SELECT 1;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("alter view") || msg.contains("alter") && msg.contains("view"),
        "message should mention ALTER VIEW, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "ALTER VIEW v AS SELECT 1;";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn two_alter_views_two_violations() {
    let sql = "ALTER VIEW v1 AS SELECT 1;\nALTER VIEW v2 AS SELECT 2;";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}
