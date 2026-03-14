use sqrust_core::FileContext;
use sqrust_rules::ambiguous::concat_function_null_arg::ConcatFunctionNullArg;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ConcatFunctionNullArg.check(&ctx(sql))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ConcatFunctionNullArg.name(), "Ambiguous/ConcatFunctionNullArg");
}

#[test]
fn concat_null_second_arg_violation() {
    let diags = check("SELECT CONCAT(a, NULL) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn concat_null_first_arg_violation() {
    let diags = check("SELECT CONCAT(NULL, a) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn concat_null_third_arg_violation() {
    let diags = check("SELECT CONCAT(a, b, NULL) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn concat_multiple_null_args_single_violation_per_call() {
    // CONCAT(NULL, NULL) should flag once — one violation per call, not per NULL
    let diags = check("SELECT CONCAT(NULL, NULL) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn concat_no_null_no_violation() {
    let diags = check("SELECT CONCAT(a, b) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn concat_with_coalesce_no_violation() {
    let diags = check("SELECT CONCAT(a, COALESCE(b, '')) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn lowercase_concat_null_violation() {
    // Case-insensitive: concat() should also be flagged
    let diags = check("SELECT concat(a, null) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn concat_in_where_clause_violation() {
    let diags = check("SELECT * FROM t WHERE CONCAT(col, NULL) = 'x'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn concat_in_cte_violation() {
    let sql = "WITH cte AS (SELECT CONCAT(a, NULL) AS v FROM t) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn concat_in_subquery_violation() {
    let sql = "SELECT * FROM (SELECT CONCAT(a, NULL) AS v FROM t) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn concat_ws_no_violation() {
    // CONCAT_WS is a different function — should not be flagged
    let diags = check("SELECT CONCAT_WS(',', a, NULL) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn message_content_is_informative() {
    let diags = check("SELECT CONCAT(a, NULL) FROM t");
    let msg = &diags[0].message;
    assert!(
        msg.contains("NULL") || msg.contains("COALESCE"),
        "message was: {msg}"
    );
}

#[test]
fn line_col_are_nonzero() {
    let diags = check("SELECT CONCAT(a, NULL) FROM t");
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
