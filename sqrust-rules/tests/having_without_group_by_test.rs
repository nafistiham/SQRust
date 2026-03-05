use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::having_without_group_by::HavingWithoutGroupBy;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    HavingWithoutGroupBy.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(HavingWithoutGroupBy.name(), "Ambiguous/HavingWithoutGroupBy");
}

#[test]
fn having_without_group_by_one_violation() {
    let diags = check("SELECT COUNT(*) FROM t HAVING COUNT(*) > 5");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/HavingWithoutGroupBy");
}

#[test]
fn having_with_group_by_no_violation() {
    let diags = check("SELECT col, COUNT(*) FROM t GROUP BY col HAVING COUNT(*) > 5");
    assert!(diags.is_empty());
}

#[test]
fn where_no_having_no_violation() {
    let diags = check("SELECT col FROM t WHERE col > 5");
    assert!(diags.is_empty());
}

#[test]
fn group_by_without_having_no_violation() {
    let diags = check("SELECT col, COUNT(*) FROM t GROUP BY col");
    assert!(diags.is_empty());
}

#[test]
fn multiple_statements_only_offender_flagged() {
    // First stmt is fine, second has HAVING without GROUP BY
    let sql = "SELECT col FROM t GROUP BY col HAVING COUNT(*) > 1; SELECT COUNT(*) FROM u HAVING COUNT(*) > 5";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn subquery_having_without_group_by_detected() {
    let sql = "SELECT id FROM (SELECT COUNT(*) cnt FROM t HAVING COUNT(*) > 5) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = HavingWithoutGroupBy.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn trivial_having_without_group_by_one_violation() {
    let diags = check("SELECT 1 FROM t HAVING 1=1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn group_by_multiple_columns_with_having_no_violation() {
    let diags = check("SELECT a, b, COUNT(*) FROM t GROUP BY a, b HAVING COUNT(*) > 2");
    assert!(diags.is_empty());
}

#[test]
fn correct_message_text() {
    let diags = check("SELECT COUNT(*) FROM t HAVING COUNT(*) > 5");
    assert_eq!(diags[0].message, "HAVING without GROUP BY; did you mean WHERE?");
}

#[test]
fn having_position_points_to_having_keyword() {
    // "SELECT COUNT(*) FROM t HAVING COUNT(*) > 5"
    //  col 1                    col 24 ^
    let diags = check("SELECT COUNT(*) FROM t HAVING COUNT(*) > 5");
    assert_eq!(diags.len(), 1);
    // Line 1, col must point to the HAVING keyword
    assert_eq!(diags[0].line, 1);
    // "HAVING" starts at byte offset 23 (0-indexed), so col = 24
    assert_eq!(diags[0].col, 24);
}

#[test]
fn having_without_group_by_multiline() {
    let sql = "SELECT COUNT(*)\nFROM t\nHAVING COUNT(*) > 5";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
    assert_eq!(diags[0].col, 1);
}

#[test]
fn two_stmts_both_having_without_group_by_two_violations() {
    let sql = "SELECT COUNT(*) FROM t HAVING COUNT(*) > 1; SELECT SUM(x) FROM u HAVING SUM(x) > 10";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}
