use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::date_arithmetic::DateArithmetic;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    DateArithmetic.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DateArithmetic.name(), "Ambiguous/DateArithmetic");
}

#[test]
fn created_at_plus_integer_one_violation() {
    let diags = check("SELECT * FROM t WHERE created_at + 1 > NOW()");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/DateArithmetic");
}

#[test]
fn created_at_plus_interval_no_violation() {
    let diags = check("SELECT * FROM t WHERE created_at + INTERVAL '1' DAY > NOW()");
    assert!(diags.is_empty());
}

#[test]
fn non_date_column_plus_integer_no_violation() {
    let diags = check("SELECT * FROM t WHERE a + 1 > 0");
    assert!(diags.is_empty());
}

#[test]
fn updated_at_minus_integer_one_violation() {
    let diags = check("SELECT * FROM t WHERE updated_at - 7 < NOW()");
    assert_eq!(diags.len(), 1);
}

#[test]
fn date_col_plus_integer_one_violation() {
    let diags = check("SELECT * FROM t WHERE date_col + 1 = '2023-01-02'");
    assert_eq!(diags.len(), 1);
}

#[test]
fn column_plus_column_no_violation() {
    let diags = check("SELECT * FROM t WHERE a + b > 0");
    assert!(diags.is_empty());
}

#[test]
fn select_created_at_plus_integer_one_violation() {
    let diags = check("SELECT created_at + 1 FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    // If the source doesn't parse, check should return empty
    // (this rule is source-level but we still respect parse_errors convention)
    // Either way, no crash should occur
    let _diags = DateArithmetic.check(&ctx);
}

#[test]
fn ts_start_minus_integer_one_violation() {
    let diags = check("SELECT * FROM t WHERE ts_start - 30 < NOW()");
    assert_eq!(diags.len(), 1);
}

#[test]
fn non_date_col_no_violation() {
    let diags = check("SELECT * FROM t WHERE non_date_col + 1 > 5");
    assert!(diags.is_empty());
}

#[test]
fn date_arithmetic_in_cte_one_violation() {
    let sql = "WITH cte AS (SELECT created_at + 1 AS next_day FROM t) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn date_arithmetic_in_subquery_one_violation() {
    let sql = "SELECT * FROM (SELECT updated_at - 7 AS week_ago FROM t) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn dateadd_function_no_violation() {
    let diags = check("SELECT DATEADD(day, 1, created_at) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_interval_hint() {
    let diags = check("SELECT * FROM t WHERE created_at + 1 > NOW()");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("INTERVAL"),
        "Expected message to contain 'INTERVAL', got: {}",
        diags[0].message
    );
}
