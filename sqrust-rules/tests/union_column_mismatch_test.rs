use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::union_column_mismatch::UnionColumnMismatch;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    UnionColumnMismatch.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnionColumnMismatch.name(), "Ambiguous/UnionColumnMismatch");
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECT FROM FROM UNION GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = UnionColumnMismatch.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn two_branches_same_count_no_violation() {
    // SELECT a, b UNION SELECT c, d — both have 2 columns
    let diags = check("SELECT a, b UNION SELECT c, d");
    assert!(diags.is_empty());
}

#[test]
fn left_has_more_columns_one_violation() {
    // SELECT a, b UNION SELECT c — 2 vs 1
    let diags = check("SELECT a, b UNION SELECT c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn right_has_more_columns_one_violation() {
    // SELECT a UNION SELECT b, c — 1 vs 2
    let diags = check("SELECT a UNION SELECT b, c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn wildcard_on_left_skipped_no_violation() {
    // SELECT * UNION SELECT a, b — wildcard branch skipped, can't determine count
    let diags = check("SELECT * UNION SELECT a, b");
    assert!(diags.is_empty());
}

#[test]
fn wildcard_on_right_skipped_no_violation() {
    // SELECT a, b UNION SELECT * — wildcard branch skipped
    let diags = check("SELECT a, b UNION SELECT *");
    assert!(diags.is_empty());
}

#[test]
fn union_all_same_count_no_violation() {
    let diags = check("SELECT a, b UNION ALL SELECT c, d");
    assert!(diags.is_empty());
}

#[test]
fn union_all_mismatched_count_one_violation() {
    let diags = check("SELECT a, b UNION ALL SELECT c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_way_union_all_consistent_no_violation() {
    let diags = check("SELECT a, b UNION SELECT c, d UNION SELECT e, f");
    assert!(diags.is_empty());
}

#[test]
fn three_way_union_one_mismatched_one_violation() {
    // Third branch has only 1 column while first two have 2
    let diags = check("SELECT a, b UNION SELECT c, d UNION SELECT e");
    assert_eq!(diags.len(), 1);
}

#[test]
fn single_select_no_union_no_violation() {
    let diags = check("SELECT a, b, c FROM t");
    assert!(diags.is_empty());
}

#[test]
fn intersect_mismatched_counts_one_violation() {
    let diags = check("SELECT a, b INTERSECT SELECT c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn except_mismatched_counts_one_violation() {
    let diags = check("SELECT a, b EXCEPT SELECT c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_column_counts() {
    let diags = check("SELECT a, b UNION SELECT c");
    assert_eq!(diags.len(), 1);
    // Message should mention the counts
    let msg = &diags[0].message;
    assert!(
        msg.contains('1') && msg.contains('2'),
        "message should contain column counts, got: {}",
        msg
    );
}

#[test]
fn three_way_last_mismatched_one_violation() {
    // a,b then c,d then e — third branch mismatch
    let diags = check("SELECT a, b UNION SELECT c, d UNION SELECT e");
    assert_eq!(diags.len(), 1);
}

#[test]
fn rule_is_assigned_to_diagnostic() {
    let diags = check("SELECT a UNION SELECT b, c");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/UnionColumnMismatch");
}
