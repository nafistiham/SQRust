use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::case_end_new_line::CaseEndNewLine;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(CaseEndNewLine.name(), "Layout/CaseEndNewLine");
}

// ── Violations ───────────────────────────────────────────────────────────────

#[test]
fn end_not_on_new_line_violation() {
    // END appears on the same line as ELSE content.
    let sql = "SELECT\nCASE WHEN x=1 THEN 'a' ELSE 'b' END\nFROM t";
    let diags = CaseEndNewLine.check(&ctx(sql));
    assert!(!diags.is_empty(), "expected violation when END is not on its own line");
}

#[test]
fn end_immediately_after_then_violation() {
    let sql = "SELECT\nCASE WHEN x > 0 THEN 'val' END\nFROM t";
    let diags = CaseEndNewLine.check(&ctx(sql));
    assert!(!diags.is_empty(), "expected violation: THEN 'val' END on same line");
}

#[test]
fn else_null_end_violation() {
    let sql = "SELECT\nCASE WHEN x IS NULL THEN 0 ELSE NULL END\nFROM t";
    let diags = CaseEndNewLine.check(&ctx(sql));
    assert!(!diags.is_empty(), "expected violation: ELSE NULL END on same line");
}

#[test]
fn multiple_case_expressions_violations() {
    // Two CASE expressions each with END on the same line as their content.
    let sql = concat!(
        "SELECT\n",
        "  CASE WHEN a = 1 THEN 'x' ELSE 'y' END,\n",
        "  CASE WHEN b = 2 THEN 'p' ELSE 'q' END\n",
        "FROM t"
    );
    let diags = CaseEndNewLine.check(&ctx(sql));
    assert!(
        diags.len() >= 2,
        "expected two violations, got {}",
        diags.len()
    );
}

// ── No violations ────────────────────────────────────────────────────────────

#[test]
fn end_on_new_line_no_violation() {
    let sql = concat!(
        "SELECT\n",
        "  CASE status\n",
        "    WHEN 1 THEN 'active'\n",
        "    WHEN 2 THEN 'inactive'\n",
        "    ELSE 'unknown'\n",
        "  END\n",
        "FROM t"
    );
    let diags = CaseEndNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations, END is on its own line (got {} violations)",
        diags.len()
    );
}

#[test]
fn single_line_no_violation() {
    // Single-line queries are exempt.
    let sql = "SELECT CASE WHEN x=1 THEN 'a' END FROM t";
    let diags = CaseEndNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations for single-line query"
    );
}

#[test]
fn end_in_string_no_violation() {
    let sql = "SELECT 'CASE WHEN 1 THEN 2 END' FROM t";
    let diags = CaseEndNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations, END is inside a string"
    );
}

#[test]
fn end_in_comment_no_violation() {
    let sql = "-- END\nSELECT 1";
    let diags = CaseEndNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations, END is inside a comment"
    );
}

#[test]
fn no_case_no_violation() {
    // No CASE keyword — rule should not fire.
    let sql = "SELECT id, name\nFROM t\nWHERE active = 1";
    let diags = CaseEndNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations when CASE is absent"
    );
}

#[test]
fn end_with_leading_whitespace_no_violation() {
    // END is the first non-whitespace token on its line.
    let sql = "SELECT\n  CASE x\n    WHEN 1 THEN 'a'\n    END\nFROM t";
    let diags = CaseEndNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations, END preceded only by whitespace"
    );
}

#[test]
fn end_at_line_start_no_violation() {
    let sql = "SELECT\n  CASE\n    WHEN score > 90 THEN 'A'\n    ELSE 'B'\nEND\nFROM t";
    let diags = CaseEndNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations, END at column 1"
    );
}

#[test]
fn empty_file_no_violation() {
    let diags = CaseEndNewLine.check(&ctx(""));
    assert!(diags.is_empty());
}
