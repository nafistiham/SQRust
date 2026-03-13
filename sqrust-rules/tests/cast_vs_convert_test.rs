use sqrust_core::FileContext;
use sqrust_rules::convention::cast_vs_convert::CastVsConvert;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CastVsConvert.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CastVsConvert.name(), "Convention/CastVsConvert");
}

#[test]
fn convert_sql_server_form_one_violation() {
    let diags = check("SELECT CONVERT(INT, col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cast_no_violation() {
    let diags = check("SELECT CAST(col AS INT) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn convert_case_insensitive_violation() {
    let diags = check("SELECT convert(varchar(50), col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn convert_mysql_form_violation() {
    let diags = check("SELECT CONVERT(col, UNSIGNED) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn convert_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE CONVERT(INT, col) > 5");
    assert_eq!(diags.len(), 1);
}

#[test]
fn nested_convert_two_violations() {
    let diags = check("SELECT CONVERT(INT, CONVERT(FLOAT, col)) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn convert_in_cte_violation() {
    let diags = check("WITH c AS (SELECT CONVERT(INT, x) AS v FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn convert_in_subquery_violation() {
    let diags = check("SELECT a FROM (SELECT CONVERT(INT, b) AS v FROM t) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cast_varchar_no_violation() {
    let diags = check("SELECT CAST(col AS VARCHAR(50)) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn try_cast_no_violation() {
    let diags = check("SELECT TRY_CAST(col AS INT) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn multiple_convert_calls_correct_count() {
    let diags = check("SELECT CONVERT(INT, a), CONVERT(VARCHAR, b), CONVERT(FLOAT, c) FROM t");
    assert_eq!(diags.len(), 3);
}

#[test]
fn message_contains_convert_or_cast() {
    let diags = check("SELECT CONVERT(INT, col) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("CONVERT") || upper.contains("CAST"),
        "message should contain 'CONVERT' or 'CAST', got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT CONVERT(INT, col) FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
