use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::window_frame_all_rows::WindowFrameAllRows;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    WindowFrameAllRows.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(WindowFrameAllRows.name(), "Structure/WindowFrameAllRows");
}

#[test]
fn rows_unbounded_preceding_following_no_partition_one_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn rows_unbounded_preceding_following_with_partition_no_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert!(diags.is_empty());
}

#[test]
fn rows_unbounded_preceding_following_with_order_by_no_partition_one_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn rows_unbounded_preceding_following_with_partition_and_order_by_no_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert!(diags.is_empty());
}

#[test]
fn rows_unbounded_preceding_current_row_no_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
    assert!(diags.is_empty());
}

#[test]
fn rows_current_row_unbounded_following_no_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert!(diags.is_empty());
}

#[test]
fn empty_over_no_violation() {
    let diags = check("SELECT SUM(val) OVER () FROM t");
    assert!(diags.is_empty());
}

#[test]
fn over_with_order_by_only_no_violation() {
    let diags = check("SELECT SUM(val) OVER (ORDER BY id) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn row_number_over_order_by_no_violation() {
    let diags = check("SELECT ROW_NUMBER() OVER (ORDER BY id) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn parse_error_no_violation() {
    let diags = check("NOT VALID SQL @@@");
    assert!(diags.is_empty());
}

#[test]
fn violation_detected_in_subquery() {
    let diags = check(
        "SELECT x FROM (SELECT SUM(val) OVER (ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t) sub",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn violation_detected_in_cte() {
    let diags = check(
        "WITH cte AS (SELECT SUM(val) OVER (ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t) SELECT * FROM cte",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_partition_by_or_entire_table() {
    let diags = check(
        "SELECT SUM(val) OVER (ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(msg.contains("PARTITION BY") || msg.contains("entire table"));
}
