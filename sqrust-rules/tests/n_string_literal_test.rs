use sqrust_core::FileContext;
use sqrust_rules::convention::n_string_literal::NStringLiteral;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NStringLiteral.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NStringLiteral.name(), "Convention/NStringLiteral");
}

#[test]
fn n_string_literal_violation() {
    let diags = check("SELECT * FROM t WHERE name = N'Alice'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_n_string_violation() {
    let diags = check("SELECT * FROM t WHERE name = n'Alice'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn regular_string_no_violation() {
    let diags = check("SELECT * FROM t WHERE name = 'Alice'");
    assert!(diags.is_empty());
}

#[test]
fn n_string_in_select_violation() {
    let diags = check("SELECT N'hello' AS greeting FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn n_string_case_insensitive() {
    let diags = check("SELECT N'hello', n'world' FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn n_string_in_column_name_no_violation() {
    let diags = check("SELECT n_name FROM t");
    assert!(diags.is_empty());
}

#[test]
fn n_string_in_comment_no_violation() {
    let diags = check("-- Use N'string' for unicode\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn multiple_n_strings_multiple_violations() {
    let diags = check("INSERT INTO t (a, b) VALUES (N'foo', N'bar')");
    assert_eq!(diags.len(), 2);
}

#[test]
fn n_string_message_content() {
    let diags = check("SELECT N'hello' FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    assert!(
        msg.contains("N'") || msg.contains("national character"),
        "message should describe N-string issue, got: {msg}"
    );
}

#[test]
fn n_string_in_insert_violation() {
    let diags = check("INSERT INTO t (name) VALUES (N'Bob')");
    assert_eq!(diags.len(), 1);
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT N'hello' FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn n_string_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE city = N'London'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn empty_n_string_violation() {
    let diags = check("SELECT N'' FROM t");
    assert_eq!(diags.len(), 1);
}
