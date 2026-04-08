use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::between_reversed_bounds::BetweenReversedBounds;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    BetweenReversedBounds.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(BetweenReversedBounds.name(), "Ambiguous/BetweenReversedBounds");
}

#[test]
fn reversed_integer_bounds_violation() {
    let diags = check("SELECT * FROM t WHERE age BETWEEN 50 AND 10");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/BetweenReversedBounds");
    assert!(diags[0].message.contains("50"));
    assert!(diags[0].message.contains("10"));
}

#[test]
fn correct_bounds_no_violation() {
    let diags = check("SELECT * FROM t WHERE age BETWEEN 10 AND 50");
    assert!(diags.is_empty());
}

#[test]
fn equal_bounds_no_violation() {
    // Equal bounds are technically an empty range but not reversed.
    let diags = check("SELECT * FROM t WHERE age BETWEEN 10 AND 10");
    assert!(diags.is_empty());
}

#[test]
fn between_in_string_no_violation() {
    let diags = check("SELECT 'BETWEEN 50 AND 10'");
    assert!(diags.is_empty());
}

#[test]
fn between_in_comment_no_violation() {
    let diags = check("-- BETWEEN 50 AND 10\nSELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn float_bounds_violation() {
    // Decimals are also detected.
    let diags = check("SELECT * FROM t WHERE score BETWEEN 5.5 AND 1.1");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/BetweenReversedBounds");
}

#[test]
fn column_bounds_no_violation() {
    // Column references as bounds — not literals, skip.
    let diags = check("SELECT * FROM t WHERE x BETWEEN col1 AND col2");
    assert!(diags.is_empty());
}

#[test]
fn negative_bounds_reversed_violation() {
    let diags = check("SELECT * FROM t WHERE x BETWEEN -1 AND -10");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/BetweenReversedBounds");
}

#[test]
fn negative_bounds_correct_no_violation() {
    let diags = check("SELECT * FROM t WHERE x BETWEEN -10 AND -1");
    assert!(diags.is_empty());
}

#[test]
fn multi_line_between_violation() {
    let sql = "SELECT *\nFROM t\nWHERE age\n  BETWEEN 100\n  AND 5";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/BetweenReversedBounds");
}

#[test]
fn multiple_between_violations() {
    let sql = "SELECT * FROM t WHERE a BETWEEN 50 AND 10 AND b BETWEEN 99 AND 1";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn no_between_keyword_no_violation() {
    let diags = check("SELECT * FROM t WHERE age > 10 AND age < 50");
    assert!(diags.is_empty());
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}
