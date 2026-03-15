use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::window_frame_full_partition::WindowFrameFullPartition;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    WindowFrameFullPartition.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        WindowFrameFullPartition.name(),
        "Structure/WindowFrameFullPartition"
    );
}

#[test]
fn rows_between_unbounded_preceding_and_unbounded_following_flagged() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn range_between_unbounded_preceding_and_unbounded_following_flagged() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp RANGE BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn rows_between_unbounded_preceding_current_row_no_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
    assert!(diags.is_empty());
}

#[test]
fn rows_between_current_row_unbounded_following_no_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert!(diags.is_empty());
}

#[test]
fn no_frame_clause_no_violation() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp ORDER BY id) FROM t",
    );
    assert!(diags.is_empty());
}

#[test]
fn rule_name_in_diagnostic() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/WindowFrameFullPartition");
}

#[test]
fn message_mentions_entire_partition() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("entire partition") || msg.contains("unbounded"),
        "expected message to mention entire partition or unbounded, got: {}",
        diags[0].message
    );
}

#[test]
fn case_insensitive_rows() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp rows between unbounded preceding and unbounded following) FROM t",
    );
    assert_eq!(diags.len(), 1, "detection should be case-insensitive");
}

#[test]
fn case_insensitive_range() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp range between unbounded preceding and unbounded following) FROM t",
    );
    assert_eq!(diags.len(), 1, "detection should be case-insensitive");
}

#[test]
fn pattern_in_string_not_flagged() {
    let diags = check(
        "SELECT 'ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING' AS note FROM t",
    );
    assert!(diags.is_empty(), "pattern in string literal should not be flagged");
}

#[test]
fn pattern_in_line_comment_not_flagged() {
    let sql = "-- ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING\nSELECT id FROM t";
    let diags = check(sql);
    assert!(diags.is_empty(), "pattern in line comment should not be flagged");
}

#[test]
fn pattern_in_block_comment_not_flagged() {
    let sql = "/* ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING */\nSELECT id FROM t";
    let diags = check(sql);
    assert!(diags.is_empty(), "pattern in block comment should not be flagged");
}

#[test]
fn multiple_occurrences_multiple_violations() {
    let diags = check(
        "SELECT SUM(a) OVER (PARTITION BY g ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING), \
         SUM(b) OVER (PARTITION BY g ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert_eq!(diags.len(), 2, "each occurrence should produce one violation");
}

#[test]
fn line_col_nonzero() {
    let diags = check(
        "SELECT SUM(val) OVER (PARTITION BY grp ROWS BETWEEN UNBOUNDED PRECEDING AND UNBOUNDED FOLLOWING) FROM t",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn empty_sql_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn no_window_function_no_violation() {
    let diags = check("SELECT id, name FROM users WHERE active = 1");
    assert!(diags.is_empty());
}

#[test]
fn unbounded_preceding_only_no_violation() {
    // Only UNBOUNDED PRECEDING with CURRENT ROW — not full partition
    let diags = check(
        "SELECT SUM(val) OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM t",
    );
    assert!(diags.is_empty());
}
