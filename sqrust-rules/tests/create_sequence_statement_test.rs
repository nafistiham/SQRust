use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::create_sequence_statement::CreateSequenceStatement;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CreateSequenceStatement.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        CreateSequenceStatement.name(),
        "Lint/CreateSequenceStatement"
    );
}

#[test]
fn create_sequence_one_violation() {
    let sql = "CREATE SEQUENCE my_seq START WITH 1 INCREMENT BY 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_sequence_lowercase_one_violation() {
    let sql = "create sequence my_seq start with 1 increment by 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_sequence_mixed_case_one_violation() {
    let sql = "Create Sequence my_seq";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_table_not_flagged() {
    let sql = "CREATE TABLE t (id INT)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_schema_not_flagged() {
    let sql = "CREATE SCHEMA my_schema";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_not_flagged() {
    let sql = "SELECT * FROM t WHERE id = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn in_string_literal_not_flagged() {
    let sql = "SELECT 'CREATE SEQUENCE my_seq' AS example FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn in_line_comment_not_flagged() {
    let sql = "-- CREATE SEQUENCE my_seq\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn in_block_comment_not_flagged() {
    let sql = "/* CREATE SEQUENCE my_seq */\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_create_sequence_two_violations() {
    let sql = "CREATE SEQUENCE seq1;\nCREATE SEQUENCE seq2;";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn word_boundary_no_false_positive() {
    // "SEQUENCE_ID" should not trigger — underscore makes it not a keyword
    let sql = "SELECT SEQUENCE_ID FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_dialect() {
    let sql = "CREATE SEQUENCE my_seq";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("dialect") || msg.contains("mysql") || msg.contains("sqlite"),
        "message should mention dialect compatibility, got: {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let sql = "CREATE SEQUENCE my_seq";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn col_nonzero() {
    let sql = "CREATE SEQUENCE my_seq";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn line_col_second_line() {
    let sql = "SELECT 1;\nCREATE SEQUENCE my_seq";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn create_sequence_with_if_not_exists_flagged() {
    // Some dialects support IF NOT EXISTS; still flagged as dialect-specific
    let sql = "CREATE SEQUENCE IF NOT EXISTS my_seq";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
