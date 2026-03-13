use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::coalesce_with_single_arg::CoalesceWithSingleArg;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CoalesceWithSingleArg.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CoalesceWithSingleArg.name(), "Ambiguous/CoalesceWithSingleArg");
}

#[test]
fn coalesce_single_arg_one_violation() {
    let diags = check("SELECT COALESCE(a) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/CoalesceWithSingleArg");
}

#[test]
fn coalesce_two_args_no_violation() {
    let diags = check("SELECT COALESCE(a, 0) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_three_args_no_violation() {
    let diags = check("SELECT COALESCE(a, b, c) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_two_args_null_fallback_no_violation() {
    let diags = check("SELECT COALESCE(a, NULL) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn coalesce_zero_args_no_violation() {
    // COALESCE() with 0 args is a parse error in most parsers — just ensure no panic
    let ctx = FileContext::from_source("SELECT COALESCE() FROM t", "test.sql");
    // If it parses (0 args), we should not flag it; if parse error, also 0 violations
    let diags = CoalesceWithSingleArg.check(&ctx);
    assert!(diags.is_empty());
}

#[test]
fn coalesce_lowercase_single_arg_one_violation() {
    let diags = check("SELECT coalesce(a) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_mixed_case_single_arg_one_violation() {
    let diags = check("SELECT coalesce(x) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_single_arg_in_where_one_violation() {
    let diags = check("SELECT * FROM t WHERE COALESCE(col) = 0");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_single_arg_in_case_one_violation() {
    let diags = check("SELECT CASE WHEN COALESCE(x) = 1 THEN 'y' END FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_single_arg_in_subquery_one_violation() {
    let diags = check("SELECT a FROM (SELECT COALESCE(x) AS c FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_single_arg_in_cte_one_violation() {
    let diags = check("WITH c AS (SELECT COALESCE(a) FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = CoalesceWithSingleArg.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn nullif_function_no_violation() {
    let diags = check("SELECT NULLIF(a, b) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_expected_text() {
    let diags = check("SELECT COALESCE(a) FROM t");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("single argument") || msg.contains("fallback"),
        "message was: {msg}"
    );
}
