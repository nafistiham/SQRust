use sqrust_core::FileContext;
use sqrust_rules::convention::coalesce_null_arg::CoalesceNullArg;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    CoalesceNullArg.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CoalesceNullArg.name(), "Convention/CoalesceNullArg");
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_null_second_arg_violation() {
    let diags = check("SELECT COALESCE(col, NULL) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_null_first_arg_violation() {
    let diags = check("SELECT COALESCE(NULL, col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_null_middle_arg_violation() {
    let diags = check("SELECT COALESCE(a, NULL, b) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_no_null_no_violation() {
    let diags = check("SELECT COALESCE(a, b, c) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_with_zero_no_violation() {
    // 0 is not NULL
    let diags = check("SELECT COALESCE(col, 0) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_with_empty_string_no_violation() {
    let diags = check("SELECT COALESCE(col, '') FROM t");
    assert!(diags.is_empty());
}

#[test]
fn non_coalesce_function_no_violation() {
    // NULLIF is a different function — should not be flagged
    let diags = check("SELECT NULLIF(a, NULL) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn two_coalesces_both_with_null_two_violations() {
    let sql = "SELECT COALESCE(a, NULL), COALESCE(NULL, b) FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn message_mentions_redundant_or_null() {
    let diags = check("SELECT COALESCE(col, NULL) FROM t");
    let msg = &diags[0].message;
    assert!(
        msg.contains("redundant") || msg.contains("NULL"),
        "message was: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT COALESCE(col, NULL) FROM t");
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn select_without_coalesce_no_violation() {
    let diags = check("SELECT name, age FROM t WHERE active = 1");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_in_where_clause_violation() {
    let diags = check("SELECT * FROM t WHERE COALESCE(col, NULL) > 0");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_points_to_coalesce_keyword() {
    // "SELECT COALESCE(col, NULL) FROM t"
    //  1234567890123456789012345678901234
    //  COALESCE starts at col 8
    let diags = check("SELECT COALESCE(col, NULL) FROM t");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 8);
}

#[test]
fn lowercase_coalesce_null_violation() {
    let diags = check("SELECT coalesce(col, null) FROM t");
    assert_eq!(diags.len(), 1);
}
