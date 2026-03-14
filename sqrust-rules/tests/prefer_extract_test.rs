use sqrust_core::FileContext;
use sqrust_rules::convention::prefer_extract::PreferExtract;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    PreferExtract.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(PreferExtract.name(), "Convention/PreferExtract");
}

#[test]
fn year_function_one_violation() {
    let diags = check("SELECT YEAR(d) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn month_function_one_violation() {
    let diags = check("SELECT MONTH(d) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn day_function_one_violation() {
    let diags = check("SELECT DAY(d) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn hour_function_one_violation() {
    let diags = check("SELECT HOUR(d) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn minute_function_one_violation() {
    let diags = check("SELECT MINUTE(d) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn second_function_one_violation() {
    let diags = check("SELECT SECOND(d) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn year_case_insensitive() {
    let diags = check("SELECT year(d) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn extract_no_violation() {
    let diags = check("SELECT EXTRACT(YEAR FROM d) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn datepart_no_violation() {
    // DATEPART is a different rule territory; PreferExtract only covers
    // the function-style YEAR()/MONTH()/etc. calls.
    let diags = check("SELECT DATEPART(year, d) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn multiple_date_functions_multiple_violations() {
    let diags = check("SELECT YEAR(d), MONTH(d), DAY(d) FROM t");
    assert_eq!(diags.len(), 3);
}

#[test]
fn year_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE YEAR(d) = 2024");
    assert_eq!(diags.len(), 1);
}

#[test]
fn year_in_cte_violation() {
    let diags = check("WITH c AS (SELECT YEAR(d) AS yr FROM t) SELECT * FROM c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_extract() {
    let diags = check("SELECT YEAR(d) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("EXTRACT"),
        "message should mention EXTRACT, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT YEAR(d) FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
