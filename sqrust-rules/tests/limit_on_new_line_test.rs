use sqrust_core::FileContext;
use sqrust_rules::layout::limit_on_new_line::LimitOnNewLine;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(LimitOnNewLine.name(), "Layout/LimitOnNewLine");
}

// ── No violation: LIMIT on its own line ──────────────────────────────────────

#[test]
fn limit_on_next_line_after_order_by_no_violation() {
    let sql = "SELECT id FROM t ORDER BY id\nLIMIT 10";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn fetch_first_on_next_line_no_violation() {
    let sql = "SELECT id FROM t ORDER BY id\nFETCH FIRST 10 ROWS ONLY";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn fetch_next_on_next_line_no_violation() {
    let sql = "SELECT id FROM t ORDER BY id\nFETCH NEXT 5 ROWS ONLY";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn limit_without_order_by_no_violation() {
    let sql = "SELECT id FROM t LIMIT 10";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn no_limit_no_order_by_no_violation() {
    let sql = "SELECT * FROM t WHERE id > 1";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Violations: LIMIT on same line as ORDER BY ────────────────────────────────

#[test]
fn limit_same_line_as_order_by_violation() {
    let sql = "SELECT id FROM t ORDER BY id LIMIT 10";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_order_by_limit_same_line_violation() {
    let sql = "select id from t order by id limit 10";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_case_order_by_limit_violation() {
    let sql = "SELECT id FROM t Order By id Limit 10";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

// ── Violations: FETCH FIRST/NEXT on same line as ORDER BY ────────────────────

#[test]
fn fetch_first_same_line_as_order_by_violation() {
    let sql = "SELECT id FROM t ORDER BY id FETCH FIRST 10 ROWS ONLY";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn fetch_next_same_line_as_order_by_violation() {
    let sql = "SELECT id FROM t ORDER BY id FETCH NEXT 5 ROWS ONLY";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_fetch_first_same_line_violation() {
    let sql = "select id from t order by id fetch first 10 rows only";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

// ── Multiple violations ───────────────────────────────────────────────────────

#[test]
fn multiple_violations_multiple_lines() {
    let sql = "SELECT a FROM t ORDER BY a LIMIT 5\nUNION ALL\nSELECT b FROM t ORDER BY b LIMIT 10";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 2);
}

// ── Line number ───────────────────────────────────────────────────────────────

#[test]
fn violation_line_number_is_correct() {
    let sql = "SELECT id\nFROM t\nORDER BY id LIMIT 10";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}

// ── Skip strings and comments ─────────────────────────────────────────────────

#[test]
fn limit_order_by_in_string_no_violation() {
    let sql = "SELECT 'ORDER BY id LIMIT 10' AS msg FROM t ORDER BY id";
    let diags = LimitOnNewLine.check(&ctx(sql));
    // ORDER BY appears outside string but without LIMIT on the same line outside string
    assert!(diags.is_empty());
}

#[test]
fn limit_order_by_in_line_comment_no_violation() {
    let sql = "SELECT id FROM t\n-- ORDER BY id LIMIT 10\nORDER BY id\nLIMIT 10";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn limit_order_by_in_block_comment_no_violation() {
    let sql = "SELECT id FROM t\n/* ORDER BY id LIMIT 10 */\nORDER BY id\nLIMIT 10";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Message content ───────────────────────────────────────────────────────────

#[test]
fn limit_violation_message_mentions_limit_and_order_by() {
    let sql = "SELECT id FROM t ORDER BY id LIMIT 10";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("LIMIT"),
        "message should mention LIMIT, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].message.contains("ORDER BY"),
        "message should mention ORDER BY, got: {}",
        diags[0].message
    );
}

#[test]
fn fetch_violation_message_mentions_fetch() {
    let sql = "SELECT id FROM t ORDER BY id FETCH FIRST 10 ROWS ONLY";
    let diags = LimitOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("FETCH"),
        "message should mention FETCH, got: {}",
        diags[0].message
    );
}

// ── Empty source ──────────────────────────────────────────────────────────────

#[test]
fn empty_source_no_violation() {
    let diags = LimitOnNewLine.check(&ctx(""));
    assert!(diags.is_empty());
}
