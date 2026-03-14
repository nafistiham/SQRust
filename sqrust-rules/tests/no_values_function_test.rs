use sqrust_core::FileContext;
use sqrust_rules::convention::no_values_function::NoValuesFunction;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoValuesFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoValuesFunction.name(), "Convention/NoValuesFunction");
}

#[test]
fn values_function_in_on_duplicate_violation() {
    let diags = check(
        "INSERT INTO t (id, val) VALUES (1, 2) ON DUPLICATE KEY UPDATE val = VALUES(val)",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn insert_values_no_violation() {
    let diags = check("INSERT INTO t (a) VALUES (1, 2)");
    assert!(diags.is_empty());
}

#[test]
fn values_function_in_set_violation() {
    let diags = check("UPDATE t SET col = VALUES(col)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn values_function_case_insensitive() {
    let diags = check(
        "INSERT INTO t (id, val) VALUES (1, 2) ON DUPLICATE KEY UPDATE val = values(val)",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_values_no_violation() {
    let diags = check("SELECT * FROM t");
    assert!(diags.is_empty());
}

#[test]
fn values_keyword_in_cte_no_violation() {
    let diags = check(
        "WITH c AS (SELECT a FROM t) INSERT INTO s VALUES (1)",
    );
    assert!(diags.is_empty());
}

#[test]
fn values_function_message_content() {
    let diags = check(
        "INSERT INTO t (id, val) VALUES (1, 2) ON DUPLICATE KEY UPDATE val = VALUES(val)",
    );
    assert!(!diags.is_empty());
    let msg = &diags[0].message;
    let upper = msg.to_uppercase();
    assert!(
        upper.contains("MYSQL") || upper.contains("MYSQL-SPECIFIC") || msg.to_lowercase().contains("mysql"),
        "message should mention MySQL, got: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let diags = check(
        "INSERT INTO t (id, val) VALUES (1, 2) ON DUPLICATE KEY UPDATE val = VALUES(val)",
    );
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn values_function_in_string_no_violation() {
    let diags = check("SELECT 'VALUES(col)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn values_function_in_comment_no_violation() {
    let diags = check("-- VALUES(col)\nSELECT a FROM t");
    assert!(diags.is_empty());
}

#[test]
fn two_values_functions_two_violations() {
    let diags = check(
        "INSERT INTO t (a, b) VALUES (1, 2) ON DUPLICATE KEY UPDATE a = VALUES(a), b = VALUES(b)",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn values_function_after_set_keyword_violation() {
    let diags = check("INSERT INTO t (id, v) VALUES (1, 2) ON DUPLICATE KEY UPDATE v = VALUES(v)");
    assert_eq!(diags.len(), 1);
}
