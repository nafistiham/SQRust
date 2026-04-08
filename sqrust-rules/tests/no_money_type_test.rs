use sqrust_core::FileContext;
use sqrust_rules::convention::no_money_type::NoMoneyType;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoMoneyType.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoMoneyType.name(), "Convention/NoMoneyType");
}

#[test]
fn money_type_violation() {
    let diags = check("CREATE TABLE t (amount MONEY)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("MONEY"));
}

#[test]
fn smallmoney_type_violation() {
    let diags = check("CREATE TABLE t (amount SMALLMONEY)");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("SMALLMONEY"));
}

#[test]
fn decimal_no_violation() {
    let diags = check("CREATE TABLE t (amount DECIMAL(18,2))");
    assert!(diags.is_empty());
}

#[test]
fn numeric_no_violation() {
    let diags = check("CREATE TABLE t (amount NUMERIC(10,4))");
    assert!(diags.is_empty());
}

#[test]
fn money_in_string_no_violation() {
    let diags = check("SELECT 'amount MONEY' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn money_in_comment_no_violation() {
    let diags = check("-- MONEY type is not portable\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn money_in_column_name_no_violation() {
    // "amount_money" is an identifier — word boundary must prevent matching
    let diags = check("SELECT amount_money FROM t");
    assert!(diags.is_empty());
}

#[test]
fn both_money_and_smallmoney_violations() {
    let sql = "CREATE TABLE t (a MONEY, b SMALLMONEY)";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn create_table_with_money_violation() {
    let sql = "CREATE TABLE orders (\n  id INT,\n  total MONEY\n)";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn money_case_insensitive() {
    let diags_upper = check("CREATE TABLE t (a MONEY)");
    assert_eq!(diags_upper.len(), 1, "MONEY (upper) should trigger");

    let diags_lower = check("CREATE TABLE t (a money)");
    assert_eq!(diags_lower.len(), 1, "money (lower) should trigger");

    let diags_mixed = check("CREATE TABLE t (a Money)");
    assert_eq!(diags_mixed.len(), 1, "Money (mixed) should trigger");
}

#[test]
fn smallmoney_case_insensitive() {
    let diags_upper = check("CREATE TABLE t (a SMALLMONEY)");
    assert_eq!(diags_upper.len(), 1, "SMALLMONEY (upper) should trigger");

    let diags_lower = check("CREATE TABLE t (a smallmoney)");
    assert_eq!(diags_lower.len(), 1, "smallmoney (lower) should trigger");

    let diags_mixed = check("CREATE TABLE t (a SmallMoney)");
    assert_eq!(diags_mixed.len(), 1, "SmallMoney (mixed) should trigger");
}
