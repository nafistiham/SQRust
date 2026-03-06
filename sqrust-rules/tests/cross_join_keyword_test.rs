use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::cross_join_keyword::CrossJoinKeyword;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CrossJoinKeyword.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CrossJoinKeyword.name(), "Ambiguous/CrossJoinKeyword");
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = CrossJoinKeyword.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn explicit_cross_join_one_violation() {
    let diags = check("SELECT * FROM a CROSS JOIN b");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/CrossJoinKeyword");
}

#[test]
fn inner_join_no_violation() {
    let diags = check("SELECT * FROM a INNER JOIN b ON a.id = b.id");
    assert!(diags.is_empty());
}

#[test]
fn left_join_no_violation() {
    let diags = check("SELECT * FROM a LEFT JOIN b ON a.id = b.id");
    assert!(diags.is_empty());
}

#[test]
fn no_join_no_violation() {
    let diags = check("SELECT * FROM a");
    assert!(diags.is_empty());
}

#[test]
fn two_cross_joins_two_violations() {
    let diags = check("SELECT * FROM a CROSS JOIN b CROSS JOIN c");
    assert_eq!(diags.len(), 2);
}

#[test]
fn cross_join_in_subquery_violation() {
    let diags = check("SELECT * FROM (SELECT * FROM a CROSS JOIN b) sub");
    assert_eq!(diags.len(), 1);
}

#[test]
fn comma_join_no_violation() {
    // Implicit comma join is a different rule (ImplicitCrossJoin), not this one
    let diags = check("SELECT * FROM a, b");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT * FROM a CROSS JOIN b");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("Cartesian") || diags[0].message.contains("cross join") || diags[0].message.contains("CROSS JOIN"),
        "message was: {}", diags[0].message);
}

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM a CROSS JOIN b");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn right_join_no_violation() {
    let diags = check("SELECT * FROM a RIGHT JOIN b ON a.id = b.id");
    assert!(diags.is_empty());
}

#[test]
fn cross_join_in_cte_violation() {
    let sql = "WITH cte AS (SELECT * FROM a CROSS JOIN b) SELECT * FROM cte";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
