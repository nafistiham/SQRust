use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::multiple_count_distinct::MultipleCountDistinct;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    MultipleCountDistinct.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        MultipleCountDistinct.name(),
        "Ambiguous/MultipleCountDistinct"
    );
}

#[test]
fn single_count_distinct_no_violation() {
    let diags = check("SELECT COUNT(DISTINCT a) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn two_count_distinct_one_violation() {
    let diags = check("SELECT COUNT(DISTINCT a), COUNT(DISTINCT b) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/MultipleCountDistinct");
}

#[test]
fn three_count_distinct_one_violation() {
    // Report once per SELECT
    let diags = check("SELECT COUNT(DISTINCT a), COUNT(DISTINCT b), COUNT(DISTINCT c) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn count_plus_count_distinct_no_violation() {
    let diags = check("SELECT COUNT(a), COUNT(DISTINCT b) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn count_star_plus_count_distinct_no_violation() {
    let diags = check("SELECT COUNT(*), COUNT(DISTINCT b) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn count_distinct_plus_sum_plus_count_distinct_one_violation() {
    let diags = check("SELECT COUNT(DISTINCT a), SUM(b), COUNT(DISTINCT c) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cte_with_multiple_count_distinct_one_violation() {
    let diags = check(
        "WITH x AS (SELECT COUNT(DISTINCT a), COUNT(DISTINCT b) FROM t) SELECT * FROM x",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn subquery_with_multiple_count_distinct_one_violation() {
    let diags = check(
        "SELECT x FROM (SELECT COUNT(DISTINCT a), COUNT(DISTINCT b) FROM t) sub",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn single_count_distinct_with_group_by_no_violation() {
    let diags = check("SELECT COUNT(DISTINCT a) FROM t GROUP BY g");
    assert!(diags.is_empty());
}

#[test]
fn two_count_distinct_with_group_by_one_violation() {
    let diags = check("SELECT COUNT(DISTINCT a), COUNT(DISTINCT b) FROM t GROUP BY g");
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = MultipleCountDistinct.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn sum_distinct_not_flagged() {
    // Only COUNT(DISTINCT) is flagged, not SUM(DISTINCT)
    let diags = check("SELECT SUM(DISTINCT a), SUM(DISTINCT b) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_aggregates_no_violation() {
    let diags = check("SELECT MAX(a), MIN(b) FROM t");
    assert!(diags.is_empty());
}
