use sqrust_core::FileContext;
use sqrust_rules::layout::join_on_new_line::JoinOnNewLine;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(JoinOnNewLine.name(), "Layout/JoinOnNewLine");
}

// ── No violation: ON on the next line ─────────────────────────────────────────

#[test]
fn on_on_next_line_no_violation() {
    let sql = "SELECT *\nFROM a\nJOIN b\n  ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn inner_join_on_next_line_no_violation() {
    let sql = "SELECT *\nFROM a\nINNER JOIN b\n  ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn left_join_on_next_line_no_violation() {
    let sql = "SELECT *\nFROM a\nLEFT JOIN b\n  ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn right_join_on_next_line_no_violation() {
    let sql = "SELECT *\nFROM a\nRIGHT JOIN b\n  ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn full_join_on_next_line_no_violation() {
    let sql = "SELECT *\nFROM a\nFULL JOIN b\n  ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn cross_join_no_on_no_violation() {
    let sql = "SELECT *\nFROM a\nCROSS JOIN b";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn no_join_at_all_no_violation() {
    let sql = "SELECT a, b FROM t WHERE a = 1";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Violations: ON on the same line as JOIN ────────────────────────────────────

#[test]
fn join_and_on_same_line_violation() {
    let sql = "SELECT *\nFROM a\nJOIN b ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn inner_join_and_on_same_line_violation() {
    let sql = "SELECT *\nFROM a\nINNER JOIN b ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn left_join_and_on_same_line_violation() {
    let sql = "SELECT *\nFROM a\nLEFT JOIN b ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn left_outer_join_and_on_same_line_violation() {
    let sql = "SELECT *\nFROM a\nLEFT OUTER JOIN b ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_violations_reported() {
    let sql = "SELECT *\nFROM a\nJOIN b ON a.id = b.id\nJOIN c ON a.id = c.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 2);
}

#[test]
fn case_insensitive_detection() {
    let sql = "SELECT *\nFROM a\njoin b on a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

// ── Skip strings and comments ──────────────────────────────────────────────────

#[test]
fn join_on_in_string_no_violation() {
    let sql = "SELECT 'JOIN b ON a.id = b.id' FROM t";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn join_on_in_line_comment_no_violation() {
    let sql = "SELECT * FROM t -- JOIN b ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn join_on_in_block_comment_no_violation() {
    let sql = "SELECT * FROM t /* JOIN b ON a.id = b.id */";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Message and position ──────────────────────────────────────────────────────

#[test]
fn violation_message_mentions_on_and_join() {
    let sql = "SELECT *\nFROM a\nJOIN b ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("ON"),
        "message should mention ON, got: {}",
        diags[0].message
    );
}

#[test]
fn violation_line_is_correct() {
    // JOIN appears on line 3
    let sql = "SELECT *\nFROM a\nJOIN b ON a.id = b.id";
    let diags = JoinOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}

#[test]
fn empty_source_no_violation() {
    let diags = JoinOnNewLine.check(&ctx(""));
    assert!(diags.is_empty());
}
