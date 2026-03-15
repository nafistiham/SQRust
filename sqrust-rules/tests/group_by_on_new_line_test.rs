use sqrust_core::FileContext;
use sqrust_rules::layout::group_by_on_new_line::GroupByOnNewLine;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(GroupByOnNewLine.name(), "Layout/GroupByOnNewLine");
}

// ── No violation: GROUP BY on its own line ────────────────────────────────────

#[test]
fn group_by_on_own_line_no_violation() {
    let sql = "SELECT dept, COUNT(*)\nFROM t\nGROUP BY dept";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn group_by_with_leading_whitespace_no_violation() {
    let sql = "SELECT dept, COUNT(*)\n  FROM t\n  GROUP BY dept";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn no_group_by_no_violation() {
    let sql = "SELECT * FROM t WHERE id = 1";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn group_by_start_of_first_line_no_violation() {
    let sql = "GROUP BY dept";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn group_by_after_having_on_next_line_no_violation() {
    let sql = "SELECT dept, COUNT(*)\nFROM t\nGROUP BY dept\nHAVING COUNT(*) > 1";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Violations: GROUP BY mid-line (preceded by non-whitespace) ────────────────

#[test]
fn group_by_after_from_same_line_violation() {
    let sql = "SELECT dept, COUNT(*) FROM t GROUP BY dept";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_group_by_same_line_violation() {
    let sql = "select dept, count(*) from t group by dept";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_case_group_by_same_line_violation() {
    let sql = "SELECT dept FROM t Group By dept";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_violations_multiple_statements() {
    let sql = "SELECT a FROM t GROUP BY a\nUNION ALL\nSELECT b FROM t GROUP BY b";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 2);
}

#[test]
fn violation_line_number_is_correct() {
    let sql = "SELECT dept\nFROM t GROUP BY dept";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

// ── Skip strings and comments ──────────────────────────────────────────────────

#[test]
fn group_by_in_string_no_violation() {
    let sql = "SELECT 'GROUP BY dept' AS msg FROM t\nGROUP BY x";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn group_by_in_line_comment_no_violation() {
    let sql = "SELECT dept\nFROM t\n-- SELECT x FROM t GROUP BY x\nGROUP BY dept";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn group_by_in_block_comment_no_violation() {
    let sql = "SELECT dept\nFROM t\n/* SELECT x GROUP BY x */\nGROUP BY dept";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Message and position ──────────────────────────────────────────────────────

#[test]
fn violation_message_mentions_group_by() {
    let sql = "SELECT dept, COUNT(*) FROM t GROUP BY dept";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("GROUP BY"),
        "message should mention GROUP BY, got: {}",
        diags[0].message
    );
}

#[test]
fn empty_source_no_violation() {
    let diags = GroupByOnNewLine.check(&ctx(""));
    assert!(diags.is_empty());
}

#[test]
fn group_by_in_subquery_same_line_violation() {
    let sql = "SELECT * FROM (SELECT dept, COUNT(*) FROM t GROUP BY dept) sub";
    let diags = GroupByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}
