use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::nullable_concat::NullableConcat;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NullableConcat.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NullableConcat.name(), "Convention/NullableConcat");
}

#[test]
fn bare_columns_concat_one_violation() {
    let sql = "SELECT a || b FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn both_sides_coalesce_guarded_no_violation() {
    let sql = "SELECT COALESCE(a, '') || COALESCE(b, '') FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn left_coalesce_right_bare_one_violation() {
    // Right side b is still bare — should flag
    let sql = "SELECT COALESCE(a, '') || b FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn both_literal_strings_no_violation() {
    let sql = "SELECT 'hello' || 'world' FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn prefix_literal_concat_bare_column_one_violation() {
    let sql = "SELECT 'prefix_' || col FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn bare_column_concat_suffix_literal_one_violation() {
    let sql = "SELECT col || '_suffix' FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn concat_in_where_clause_one_violation() {
    let sql = "SELECT x FROM t WHERE a || b = 'foo'";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_concat_operator_no_violation() {
    let sql = "SELECT a, b FROM t WHERE a = b";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn ifnull_guarded_both_sides_no_violation() {
    let sql = "SELECT IFNULL(a, '') || IFNULL(b, '') FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn nvl_guarded_both_sides_no_violation() {
    let sql = "SELECT NVL(a, '') || NVL(b, '') FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn parse_error_returns_empty() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = NullableConcat.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn chained_concat_bare_columns_at_least_one_violation() {
    let sql = "SELECT col1 || col2 || col3 FROM t";
    let diags = check(sql);
    assert!(!diags.is_empty());
}

#[test]
fn message_mentions_null_and_coalesce() {
    let sql = "SELECT a || b FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("NULL"));
    assert!(diags[0].message.contains("COALESCE"));
}
