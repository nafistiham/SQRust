use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::truncate_table::TruncateTable;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    TruncateTable.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(TruncateTable.name(), "Lint/TruncateTable");
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "TRUNCATE INVALID @@## GARBAGE";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = TruncateTable.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn truncate_table_one_violation() {
    let sql = "TRUNCATE TABLE users";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_no_violation() {
    let sql = "SELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn delete_no_violation() {
    let sql = "DELETE FROM t";
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
fn two_truncate_two_violations() {
    let sql = "TRUNCATE TABLE orders;\nTRUNCATE TABLE items";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn truncate_with_table_name_violation() {
    let sql = "TRUNCATE TABLE orders";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_no_violation() {
    let sql = "INSERT INTO t VALUES (1)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn update_no_violation() {
    let sql = "UPDATE t SET a = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_contains_useful_text() {
    let sql = "TRUNCATE TABLE users";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("TRUNCATE") || diags[0].message.contains("irreversible"),
        "expected message to mention TRUNCATE or irreversible, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "TRUNCATE TABLE users";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn truncate_without_table_keyword_if_supported() {
    // GenericDialect supports TRUNCATE without the TABLE keyword.
    let sql = "TRUNCATE users";
    let ctx = FileContext::from_source(sql, "test.sql");
    // If parsing succeeds, it should produce exactly 1 violation.
    if ctx.parse_errors.is_empty() {
        let diags = TruncateTable.check(&ctx);
        assert_eq!(diags.len(), 1);
    }
    // If the dialect does not support this form and parsing fails, we accept 0 violations.
}

#[test]
fn truncate_line_number_on_second_line() {
    let sql = "SELECT 1;\nTRUNCATE TABLE users";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}
