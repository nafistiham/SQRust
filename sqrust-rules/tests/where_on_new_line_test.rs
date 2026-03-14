use sqrust_core::FileContext;
use sqrust_rules::layout::where_on_new_line::WhereOnNewLine;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(WhereOnNewLine.name(), "Layout/WhereOnNewLine");
}

// ── No violation: WHERE on its own line ───────────────────────────────────────

#[test]
fn where_on_next_line_no_violation() {
    let sql = "SELECT *\nFROM t\nWHERE id = 1";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn from_and_where_on_separate_lines_no_violation() {
    let sql = "SELECT a, b\nFROM my_table\nWHERE a > 0 AND b < 10";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn no_where_at_all_no_violation() {
    let sql = "SELECT a, b FROM t ORDER BY a";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn where_alone_on_line_no_violation() {
    let sql = "SELECT *\nFROM t\nWHERE\n  id = 1";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn multiline_from_where_on_own_line_no_violation() {
    let sql = "SELECT *\nFROM t AS t1\n  JOIN u AS t2 ON t1.id = t2.id\nWHERE t1.active = 1";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Violations: WHERE on same line as FROM ────────────────────────────────────

#[test]
fn from_and_where_same_line_violation() {
    let sql = "SELECT * FROM t WHERE id = 1";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn lowercase_from_where_same_line_violation() {
    let sql = "select * from t where id = 1";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_case_from_where_same_line_violation() {
    let sql = "SELECT * From t Where id = 1";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_violations_multiple_lines() {
    let sql = "SELECT * FROM a WHERE id = 1\nUNION ALL\nSELECT * FROM b WHERE id = 2";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 2);
}

#[test]
fn violation_line_number_is_correct() {
    let sql = "SELECT *\nFROM t WHERE id = 1";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

// ── Skip strings and comments ──────────────────────────────────────────────────

#[test]
fn from_where_in_string_no_violation() {
    let sql = "SELECT 'FROM t WHERE id = 1' AS msg FROM t";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn from_where_in_line_comment_no_violation() {
    let sql = "SELECT *\nFROM t\n-- FROM t WHERE id = 1\nWHERE id = 2";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

#[test]
fn from_where_in_block_comment_no_violation() {
    let sql = "SELECT * /* FROM t WHERE id = 1 */\nFROM t\nWHERE id = 2";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert!(diags.is_empty());
}

// ── Message and position ──────────────────────────────────────────────────────

#[test]
fn violation_message_mentions_where_and_from() {
    let sql = "SELECT * FROM t WHERE id = 1";
    let diags = WhereOnNewLine.check(&ctx(sql));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("WHERE"),
        "message should mention WHERE, got: {}",
        diags[0].message
    );
    assert!(
        diags[0].message.contains("FROM"),
        "message should mention FROM, got: {}",
        diags[0].message
    );
}

#[test]
fn empty_source_no_violation() {
    let diags = WhereOnNewLine.check(&ctx(""));
    assert!(diags.is_empty());
}
