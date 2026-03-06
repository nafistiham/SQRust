use sqrust_core::FileContext;
use sqrust_rules::convention::concat_operator::ConcatOperator;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ConcatOperator.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ConcatOperator.name(), "Convention/ConcatOperator");
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn concat_operator_one_violation() {
    let diags = check("SELECT a || b FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn concat_function_no_violation() {
    let diags = check("SELECT CONCAT(a, b) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_concat_no_violation() {
    let diags = check("SELECT a + b FROM t");
    assert!(diags.is_empty());
}

#[test]
fn concat_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE a || b = 'foo'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_concat_multiple_violations() {
    // Two || operators: a || b || c
    let diags = check("SELECT a || b || c FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn concat_in_select_and_where_two_violations() {
    let diags = check("SELECT a || b FROM t WHERE c || d = 'x'");
    assert_eq!(diags.len(), 2);
}

#[test]
fn string_literal_concat_violation() {
    let diags = check("SELECT 'foo' || 'bar' FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT a || b FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("CONCAT") || diags[0].message.contains("||"),
        "message was: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT a || b FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn update_with_concat_violation() {
    let diags = check("UPDATE t SET col = a || b WHERE id = 1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_string_operation_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

#[test]
fn concat_points_to_pipe_position() {
    // "SELECT a || b FROM t"
    //  123456789012
    //           ^col 10 = first | of ||
    let diags = check("SELECT a || b FROM t");
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 10);
}
