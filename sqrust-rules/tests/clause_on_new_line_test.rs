use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::clause_on_new_line::ClauseOnNewLine;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ClauseOnNewLine.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ClauseOnNewLine.name(), "Layout/ClauseOnNewLine");
}

#[test]
fn single_line_query_no_violation() {
    // No newlines — single-line query is always fine
    let diags = check("SELECT id FROM t WHERE id = 1");
    assert!(diags.is_empty(), "single-line query should produce no violations");
}

#[test]
fn multiline_from_on_own_line_no_violation() {
    let diags = check("SELECT id\nFROM t");
    assert!(diags.is_empty(), "FROM on its own line should not be flagged");
}

#[test]
fn multiline_from_mid_line_one_violation() {
    // FROM appears mid-line after SELECT id — should be flagged
    let diags = check("SELECT id FROM t\nWHERE id = 1");
    assert_eq!(diags.len(), 1, "FROM mid-line should produce 1 violation");
    assert!(
        diags[0].message.contains("FROM"),
        "violation message should mention FROM"
    );
}

#[test]
fn multiline_where_on_own_line_no_violation() {
    let diags = check("SELECT id\nFROM t\nWHERE id = 1");
    assert!(diags.is_empty(), "all clauses on own lines — 0 violations");
}

#[test]
fn multiline_where_mid_line_one_violation() {
    // WHERE appears after non-whitespace on the same line
    let diags = check("SELECT id\nFROM t WHERE id = 1");
    assert_eq!(diags.len(), 1, "WHERE mid-line should produce 1 violation");
    assert!(
        diags[0].message.contains("WHERE"),
        "violation message should mention WHERE"
    );
}

#[test]
fn order_by_on_own_line_no_violation() {
    let diags = check("SELECT id\nFROM t\nORDER BY id");
    assert!(diags.is_empty(), "ORDER BY on its own line — 0 violations");
}

#[test]
fn order_by_mid_line_one_violation() {
    let diags = check("SELECT id\nFROM t ORDER BY id");
    assert_eq!(diags.len(), 1, "ORDER BY mid-line should produce 1 violation");
    assert!(
        diags[0].message.contains("ORDER BY"),
        "violation message should mention ORDER BY"
    );
}

#[test]
fn group_by_on_own_line_no_violation() {
    let diags = check("SELECT id\nFROM t\nGROUP BY id");
    assert!(diags.is_empty(), "GROUP BY on its own line — 0 violations");
}

#[test]
fn having_mid_line_one_violation() {
    let diags = check("SELECT id\nFROM t\nGROUP BY id HAVING COUNT(*) > 1");
    assert_eq!(diags.len(), 1, "HAVING mid-line should produce 1 violation");
    assert!(
        diags[0].message.contains("HAVING"),
        "violation message should mention HAVING"
    );
}

#[test]
fn limit_mid_line_one_violation() {
    let diags = check("SELECT id\nFROM t\nORDER BY id LIMIT 10");
    assert_eq!(diags.len(), 1, "LIMIT mid-line should produce 1 violation");
    assert!(
        diags[0].message.contains("LIMIT"),
        "violation message should mention LIMIT"
    );
}

#[test]
fn message_contains_clause_name() {
    let diags = check("SELECT id\nFROM t WHERE id = 1");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("WHERE"),
        "message should contain the clause keyword name"
    );
}

#[test]
fn line_nonzero() {
    // WHERE mid-line appears on line 2
    let diags = check("SELECT id\nFROM t WHERE id = 1");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line should be 1-indexed");
    assert_eq!(diags[0].line, 2);
}

#[test]
fn col_nonzero() {
    // "FROM t " is 7 chars, WHERE starts at col 8 on line 2
    let diags = check("SELECT id\nFROM t WHERE id = 1");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1, "col should be 1-indexed");
    assert_eq!(diags[0].col, 8);
}
