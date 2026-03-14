use sqrust_core::FileContext;
use sqrust_rules::convention::try_cast::TryCast;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    TryCast.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(TryCast.name(), "Convention/TryCast");
}

#[test]
fn try_cast_one_violation() {
    let diags = check("SELECT TRY_CAST(x AS INT) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn safe_cast_one_violation() {
    let diags = check("SELECT SAFE_CAST(x AS INT) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn try_cast_case_insensitive() {
    let diags = check("SELECT try_cast(x AS INT) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn safe_cast_case_insensitive() {
    let diags = check("SELECT safe_cast(x AS INT) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn regular_cast_no_violation() {
    let diags = check("SELECT CAST(x AS INT) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn try_cast_in_select_violation() {
    let diags = check("SELECT TRY_CAST(amount AS DECIMAL) AS amt FROM orders");
    assert_eq!(diags.len(), 1);
}

#[test]
fn try_cast_in_where_violation() {
    let diags = check("SELECT * FROM t WHERE TRY_CAST(val AS INT) > 0");
    assert_eq!(diags.len(), 1);
}

#[test]
fn safe_cast_in_cte_violation() {
    let diags = check(
        "WITH cte AS (SELECT SAFE_CAST(x AS INT) AS v FROM t) SELECT * FROM cte",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_try_cast_two_violations() {
    let diags = check("SELECT TRY_CAST(a AS INT), TRY_CAST(b AS FLOAT) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn try_cast_and_safe_cast_two_violations() {
    let diags = check("SELECT TRY_CAST(a AS INT), SAFE_CAST(b AS FLOAT) FROM t");
    assert_eq!(diags.len(), 2);
}

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM FROM");
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_sql_server() {
    let diags = check("SELECT TRY_CAST(x AS INT) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    assert!(
        msg.contains("SQL Server") || msg.contains("Azure"),
        "message should mention SQL Server or Azure, got: {msg}"
    );
}

#[test]
fn safe_cast_message_mentions_bigquery() {
    let diags = check("SELECT SAFE_CAST(x AS INT) FROM t");
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    assert!(
        msg.contains("BigQuery"),
        "message should mention BigQuery, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT TRY_CAST(x AS INT) FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
