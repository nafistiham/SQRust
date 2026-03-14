use sqrust_core::FileContext;
use sqrust_rules::layout::having_on_new_line::HavingOnNewLine;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(HavingOnNewLine.name(), "Layout/HavingOnNewLine");
}

// ── No violation: HAVING on its own line ──────────────────────────────────────

#[test]
fn having_on_next_line_no_violation() {
    let sql = "SELECT dept, COUNT(*)\nFROM t\nGROUP BY dept\nHAVING COUNT(*) > 1";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn group_by_and_having_on_separate_lines_no_violation() {
    let sql = "SELECT region, SUM(sales)\nFROM orders\nGROUP BY region\nHAVING SUM(sales) > 100";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn no_having_at_all_no_violation() {
    let sql = "SELECT dept, COUNT(*) FROM t GROUP BY dept";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn no_group_by_at_all_no_violation() {
    let sql = "SELECT * FROM t WHERE id = 1";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn having_alone_on_line_no_violation() {
    let sql = "SELECT dept, COUNT(*)\nFROM t\nGROUP BY dept\nHAVING\n  COUNT(*) > 1";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Violations: HAVING on same line as GROUP BY ───────────────────────────────

#[test]
fn group_by_and_having_same_line_violation() {
    let sql = "SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > 1";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_group_by_having_same_line_violation() {
    let sql = "select dept, count(*) from t group by dept having count(*) > 1";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_case_group_by_having_same_line_violation() {
    let sql = "SELECT dept FROM t Group By dept Having count(*) > 1";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_violations_multiple_lines() {
    let sql = "SELECT a, COUNT(*) FROM t GROUP BY a HAVING COUNT(*) > 1\nUNION ALL\nSELECT b, COUNT(*) FROM t GROUP BY b HAVING COUNT(*) > 2";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 2);
}

#[test]
fn violation_line_number_is_correct() {
    let sql = "SELECT dept\nFROM t\nGROUP BY dept HAVING COUNT(*) > 1";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}

// ── Skip strings and comments ──────────────────────────────────────────────────

#[test]
fn group_by_having_in_string_no_violation() {
    let sql = "SELECT 'GROUP BY dept HAVING COUNT(*) > 1' AS msg FROM t GROUP BY x";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn group_by_having_in_line_comment_no_violation() {
    let sql = "SELECT dept\nFROM t\nGROUP BY dept\n-- GROUP BY x HAVING COUNT(*) > 1\nHAVING COUNT(*) > 1";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn group_by_having_in_block_comment_no_violation() {
    let sql = "SELECT dept\nFROM t\n/* GROUP BY dept HAVING COUNT(*) > 1 */\nGROUP BY dept\nHAVING COUNT(*) > 1";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Message and position ──────────────────────────────────────────────────────

#[test]
fn violation_message_mentions_having_and_group_by() {
    let sql = "SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > 1";
    let diags = HavingOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("HAVING"),
        "message should mention HAVING, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].message.contains("GROUP BY"),
        "message should mention GROUP BY, got: {}",
        diags[0].message
    );
}

#[test]
fn empty_source_no_violation() {
    let diags = HavingOnNewLine.check(&ctx(""));
    assert!(diags.is_empty());
}
