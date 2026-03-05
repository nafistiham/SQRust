use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::join_without_condition::JoinWithoutCondition;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    JoinWithoutCondition.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(JoinWithoutCondition.name(), "Ambiguous/JoinWithoutCondition");
}

#[test]
fn inner_join_no_condition_one_violation() {
    let diags = check("SELECT * FROM t1 INNER JOIN t2");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/JoinWithoutCondition");
}

#[test]
fn inner_join_with_on_no_violation() {
    let diags = check("SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id");
    assert!(diags.is_empty());
}

#[test]
fn left_join_with_on_no_violation() {
    let diags = check("SELECT * FROM t1 LEFT JOIN t2 ON t1.id = t2.id");
    assert!(diags.is_empty());
}

#[test]
fn left_join_no_condition_one_violation() {
    let diags = check("SELECT * FROM t1 LEFT JOIN t2");
    assert_eq!(diags.len(), 1);
}

#[test]
fn right_join_no_condition_one_violation() {
    let diags = check("SELECT * FROM t1 RIGHT JOIN t2");
    assert_eq!(diags.len(), 1);
}

#[test]
fn full_join_no_condition_one_violation() {
    let diags = check("SELECT * FROM t1 FULL JOIN t2");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cross_join_no_violation() {
    // CROSS JOIN intentionally has no condition
    let diags = check("SELECT * FROM t1 CROSS JOIN t2");
    assert!(diags.is_empty());
}

#[test]
fn join_using_no_violation() {
    let diags = check("SELECT * FROM t1 JOIN t2 USING (id)");
    assert!(diags.is_empty());
}

#[test]
fn multiple_joins_only_one_without_condition_one_violation() {
    let diags =
        check("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id LEFT JOIN t3");
    assert_eq!(diags.len(), 1);
}

#[test]
fn subquery_from_with_join_without_condition_detected() {
    // The inner subquery has a JOIN without condition — should be detected
    let sql = "SELECT * FROM (SELECT * FROM t1 INNER JOIN t2) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_empty() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = JoinWithoutCondition.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn message_format_is_correct() {
    let diags = check("SELECT * FROM t1 INNER JOIN t2");
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "JOIN without ON or USING condition; this will produce a cross join"
    );
}

#[test]
fn plain_join_without_condition_one_violation() {
    // bare JOIN (parsed as INNER JOIN by most parsers)
    let diags = check("SELECT * FROM t1 JOIN t2");
    assert_eq!(diags.len(), 1);
}
