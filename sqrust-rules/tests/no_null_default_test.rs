use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::no_null_default::NoNullDefault;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    NoNullDefault.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoNullDefault.name(), "Convention/NoNullDefault");
}

#[test]
fn no_default_no_violation() {
    let diags = check("CREATE TABLE t (id INT NOT NULL)");
    assert_eq!(diags.len(), 0);
}

#[test]
fn default_value_no_violation() {
    let diags = check("CREATE TABLE t (id INT DEFAULT 0)");
    assert_eq!(diags.len(), 0);
}

#[test]
fn default_null_one_violation() {
    let diags = check("CREATE TABLE t (id INT DEFAULT NULL)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn default_null_case_insensitive() {
    let diags = check("CREATE TABLE t (id INT default null)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_default_null_two_violations() {
    let diags = check(
        "CREATE TABLE t (id INT DEFAULT NULL, name VARCHAR(50) DEFAULT NULL)",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn default_null_in_string_not_flagged() {
    let diags = check("SELECT * FROM t WHERE note = 'DEFAULT NULL'");
    assert_eq!(diags.len(), 0);
}

#[test]
fn default_null_in_comment_not_flagged() {
    let diags = check("SELECT 1 -- DEFAULT NULL\nFROM t");
    assert_eq!(diags.len(), 0);
}

#[test]
fn message_content() {
    let diags = check("CREATE TABLE t (id INT DEFAULT NULL)");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("redundant") || msg.contains("NULL"),
        "message should mention 'redundant' or 'NULL', got: {msg}"
    );
}

#[test]
fn line_nonzero() {
    let diags = check("CREATE TABLE t (id INT DEFAULT NULL)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn col_nonzero() {
    let diags = check("CREATE TABLE t (id INT DEFAULT NULL)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn col_points_to_default_keyword() {
    // "  col INT DEFAULT NULL"
    //  col starts at offset 0 of this sub-string; full line shown below.
    // "CREATE TABLE t (  col INT DEFAULT NULL)"
    //  position:          123456789...
    // We will test a simpler case: single-line where DEFAULT is at a known column.
    // "CREATE TABLE t (col INT DEFAULT NULL)"
    //  1234567890123456789012345678901234567
    //  D of DEFAULT is at index 24 (0-based), so col = 25
    let sql = "CREATE TABLE t (col INT DEFAULT NULL)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    // find where DEFAULT starts
    let expected_col = sql.find("DEFAULT").unwrap() + 1; // 1-indexed
    assert_eq!(diags[0].col, expected_col);
}

#[test]
fn alter_table_default_null_flagged() {
    let diags = check("ALTER TABLE t ALTER COLUMN c SET DEFAULT NULL");
    assert_eq!(diags.len(), 1);
}
