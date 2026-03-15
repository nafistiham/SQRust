use sqrust_core::FileContext;
use sqrust_rules::layout::order_by_on_new_line::OrderByOnNewLine;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(OrderByOnNewLine.name(), "Layout/OrderByOnNewLine");
}

// ── No violation: ORDER BY on its own line ────────────────────────────────────

#[test]
fn order_by_on_next_line_after_where_no_violation() {
    let sql = "SELECT id FROM t WHERE id > 1\nORDER BY id";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn order_by_on_next_line_after_group_by_no_violation() {
    let sql = "SELECT dept, COUNT(*) FROM t\nGROUP BY dept\nORDER BY dept";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn order_by_on_next_line_after_having_no_violation() {
    let sql = "SELECT dept, COUNT(*) FROM t\nGROUP BY dept\nHAVING COUNT(*) > 1\nORDER BY dept";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn no_order_by_at_all_no_violation() {
    let sql = "SELECT * FROM t WHERE id = 1";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn order_by_alone_no_preceding_clause_no_violation() {
    let sql = "SELECT id FROM t\nORDER BY id DESC";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Violations: ORDER BY on same line as WHERE ────────────────────────────────

#[test]
fn order_by_same_line_as_where_violation() {
    let sql = "SELECT id FROM t WHERE id > 1 ORDER BY id";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_order_by_where_same_line_violation() {
    let sql = "select id from t where id > 1 order by id";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

// ── Violations: ORDER BY on same line as GROUP BY ─────────────────────────────

#[test]
fn order_by_same_line_as_group_by_violation() {
    let sql = "SELECT dept, COUNT(*) FROM t GROUP BY dept ORDER BY dept";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_group_by_order_by_same_line_violation() {
    let sql = "select dept, count(*) from t group by dept order by dept";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

// ── Violations: ORDER BY on same line as HAVING ───────────────────────────────

#[test]
fn order_by_same_line_as_having_violation() {
    let sql = "SELECT dept, COUNT(*) FROM t GROUP BY dept HAVING COUNT(*) > 1 ORDER BY dept";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_case_having_order_by_same_line_violation() {
    let sql = "SELECT dept FROM t Group By dept Having count(*) > 1 Order By dept";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

// ── Multiple violations ───────────────────────────────────────────────────────

#[test]
fn multiple_violations_multiple_lines() {
    let sql = "SELECT a FROM t WHERE a > 1 ORDER BY a\nUNION ALL\nSELECT b FROM t WHERE b > 2 ORDER BY b";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 2);
}

// ── Line number and column ────────────────────────────────────────────────────

#[test]
fn violation_line_number_is_correct() {
    let sql = "SELECT id\nFROM t\nWHERE id > 1 ORDER BY id";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}

// ── Skip strings and comments ─────────────────────────────────────────────────

#[test]
fn order_by_where_in_string_no_violation() {
    let sql = "SELECT 'WHERE x > 1 ORDER BY x' AS msg FROM t ORDER BY id";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    // ORDER BY appears outside the string but without WHERE/GROUP BY/HAVING on same line
    assert!(diags.is_empty());
}

#[test]
fn order_by_where_in_line_comment_no_violation() {
    let sql = "SELECT id FROM t\n-- WHERE id > 1 ORDER BY id\nORDER BY id";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn order_by_where_in_block_comment_no_violation() {
    let sql = "SELECT id FROM t\n/* WHERE id > 1 ORDER BY id */\nWHERE id > 0\nORDER BY id";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Message content ───────────────────────────────────────────────────────────

#[test]
fn violation_message_mentions_order_by() {
    let sql = "SELECT id FROM t WHERE id > 1 ORDER BY id";
    let diags = OrderByOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("ORDER BY"),
        "message should mention ORDER BY, got: {}",
        diags[0].message
    );
}

// ── Empty source ──────────────────────────────────────────────────────────────

#[test]
fn empty_source_no_violation() {
    let diags = OrderByOnNewLine.check(&ctx(""));
    assert!(diags.is_empty());
}
