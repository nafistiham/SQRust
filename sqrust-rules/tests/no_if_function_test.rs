use sqrust_core::FileContext;
use sqrust_rules::convention::no_if_function::NoIFFunction;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoIFFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoIFFunction.name(), "Convention/NoIFFunction");
}

#[test]
fn if_function_one_violation() {
    let diags = check("SELECT IF(a > 1, 'yes', 'no') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn if_case_insensitive() {
    let diags = check("SELECT if(a > 1, 'yes', 'no') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn if_in_select_violation() {
    let diags = check("SELECT IF(x = 1, 'a', 'b'), IF(y = 2, 'c', 'd') FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn if_in_where_violation() {
    // IF used inside WHERE clause (as a subexpression).
    let diags = check("SELECT * FROM t WHERE IF(a, 1, 0) = 1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn if_in_cte_violation() {
    let diags = check("WITH c AS (SELECT IF(a, 'x', 'y') AS v FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nested_if_two_violations() {
    let diags = check("SELECT IF(a, IF(b, 1, 2), 3) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn case_when_no_violation() {
    let diags = check("SELECT CASE WHEN a > 1 THEN 'yes' ELSE 'no' END FROM t");
    assert!(diags.is_empty());
}

#[test]
fn ifnull_no_violation() {
    // IFNULL is a different function and handled by a separate rule.
    let diags = check("SELECT IFNULL(a, 0) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_no_violation() {
    let diags = check("SELECT COALESCE(a, b, 0) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiple_if_calls_multiple_violations() {
    let diags = check("SELECT IF(a, 1, 2), IF(b, 3, 4), IF(c, 5, 6) FROM t");
    assert_eq!(diags.len(), 3);
}

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_case_when() {
    let diags = check("SELECT IF(a, 1, 0) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("CASE") && upper.contains("WHEN"),
        "message should mention CASE WHEN, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT IF(a, 1, 0) FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
