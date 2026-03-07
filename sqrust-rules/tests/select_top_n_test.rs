use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::select_top_n::SelectTopN;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    SelectTopN.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectTopN.name(), "Convention/SelectTopN");
}

#[test]
fn select_top_no_violation_on_plain_select() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

#[test]
fn select_top_flags_top_n() {
    let diags = check("SELECT TOP 10 id FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Convention/SelectTopN");
}

#[test]
fn select_top_flags_top_in_subquery() {
    let diags = check("SELECT * FROM (SELECT TOP 5 id FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_top_distinct_flagged() {
    // T-SQL: SELECT TOP N DISTINCT ... — TOP comes after SELECT
    let diags = check("SELECT TOP 10 DISTINCT id FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_top_in_string_not_flagged() {
    let diags = check("SELECT 'SELECT TOP 10' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn select_top_in_comment_not_flagged() {
    let diags = check("SELECT id FROM t -- SELECT TOP 10 here");
    assert!(diags.is_empty());
}

#[test]
fn select_top_two_statements_two_violations() {
    let diags = check("SELECT TOP 5 id FROM a; SELECT TOP 10 name FROM b");
    assert_eq!(diags.len(), 2);
}

#[test]
fn select_top_message_content() {
    let diags = check("SELECT TOP 10 id FROM t");
    let msg = &diags[0].message;
    let has_top = msg.contains("TOP");
    let has_limit = msg.contains("LIMIT");
    assert!(
        has_top || has_limit,
        "message should mention TOP or LIMIT, got: {msg}"
    );
}

#[test]
fn select_top_line_nonzero() {
    let diags = check("SELECT TOP 10 id FROM t");
    assert!(diags[0].line >= 1);
}

#[test]
fn select_top_col_nonzero() {
    let diags = check("SELECT TOP 10 id FROM t");
    assert!(diags[0].col >= 1);
}

#[test]
fn select_top_col_points_to_top_keyword() {
    // "SELECT TOP 10 id FROM t"
    //  1234567890
    // Position of 'T' in TOP is byte offset 7 (0-indexed) => col 8 (1-indexed)
    let diags = check("SELECT TOP 10 id FROM t");
    assert_eq!(diags[0].col, 8, "col should point to the 'T' of TOP");
}

#[test]
fn select_top_lowercase_flagged() {
    let diags = check("select top 10 id from t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_top_word_boundary_not_flagged() {
    // STOP and ATOP must not be flagged as TOP
    let diags = check("SELECT STOP, ATOP FROM t");
    assert!(diags.is_empty());
}
