use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::when_on_new_line::WhenOnNewLine;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(WhenOnNewLine.name(), "Layout/WhenOnNewLine");
}

// ── Violations ───────────────────────────────────────────────────────────────

#[test]
fn when_not_on_new_line_violation() {
    // WHEN appears in the middle of a line (same line as CASE or previous WHEN).
    let sql = "SELECT\nCASE status WHEN 1 THEN 'a' WHEN 2 THEN 'b' END\nFROM t";
    let diags = WhenOnNewLine.check(&ctx(sql));
    // Two WHEN occurrences both have non-whitespace content before them.
    assert!(!diags.is_empty(), "expected at least one violation");
}

#[test]
fn multiple_violations() {
    // Three WHEN clauses on the same line; all should be flagged except possibly
    // the first if it sits right after CASE keyword with no prefix on the line.
    // Here the line starts with "CASE status WHEN..." so the first WHEN has
    // "CASE status " before it — that is non-whitespace.
    let sql = "SELECT x,\nCASE col WHEN 1 THEN 'a' WHEN 2 THEN 'b' WHEN 3 THEN 'c' END\nFROM t";
    let diags = WhenOnNewLine.check(&ctx(sql));
    assert!(
        diags.len() >= 2,
        "expected multiple violations, got {}",
        diags.len()
    );
}

#[test]
fn when_in_subquery_on_same_line_violation() {
    let sql = "SELECT *\nFROM (SELECT CASE x WHEN 1 THEN 'a' END AS v FROM t) sub\nWHERE v IS NOT NULL";
    let diags = WhenOnNewLine.check(&ctx(sql));
    assert!(!diags.is_empty(), "expected violation inside subquery");
}

// ── No violations ────────────────────────────────────────────────────────────

#[test]
fn when_on_new_line_no_violation() {
    let sql =
        "SELECT\n  CASE status\n    WHEN 1 THEN 'active'\n    WHEN 2 THEN 'inactive'\n  END\nFROM t";
    let diags = WhenOnNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations, got: {} violations",
        diags.len()
    );
}

#[test]
fn single_line_query_no_violation() {
    // Single-line queries are exempt regardless of WHEN content.
    let sql = "SELECT CASE WHEN x=1 THEN 'a' END FROM t";
    let diags = WhenOnNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations for single-line query"
    );
}

#[test]
fn when_in_string_no_violation() {
    let sql = "SELECT 'CASE WHEN 1 THEN 2 END' FROM t";
    let diags = WhenOnNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations, WHEN is inside a string"
    );
}

#[test]
fn when_in_comment_no_violation() {
    let sql = "-- WHEN 1 THEN 2\nSELECT 1";
    let diags = WhenOnNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations, WHEN is inside a comment"
    );
}

#[test]
fn no_case_no_violation() {
    // No CASE keyword present — rule should not fire even if WHEN appears.
    let sql = "SELECT WHEN FROM t\nGROUP BY id";
    let diags = WhenOnNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations when CASE is absent"
    );
}

#[test]
fn when_at_line_start_no_violation() {
    // WHEN preceded only by whitespace on its line.
    let sql = "SELECT\n  CASE status\n  WHEN 1 THEN 'a'\n  END\nFROM t";
    let diags = WhenOnNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations, WHEN is at line start (after whitespace)"
    );
}

#[test]
fn when_preceded_by_whitespace_only_no_violation() {
    // Explicit \n + leading spaces before WHEN.
    let sql = "SELECT\n  CASE\n    WHEN x = 1 THEN 'yes'\n  END\nFROM t";
    let diags = WhenOnNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations, line has only whitespace before WHEN"
    );
}

#[test]
fn case_with_when_on_separate_lines_no_violation() {
    let sql = concat!(
        "SELECT\n",
        "  CASE\n",
        "    WHEN score >= 90 THEN 'A'\n",
        "    WHEN score >= 80 THEN 'B'\n",
        "    WHEN score >= 70 THEN 'C'\n",
        "    ELSE 'F'\n",
        "  END AS grade\n",
        "FROM students"
    );
    let diags = WhenOnNewLine.check(&ctx(sql));
    assert!(
        diags.is_empty(),
        "expected no violations, all WHEN on separate lines"
    );
}

#[test]
fn empty_file_no_violation() {
    let diags = WhenOnNewLine.check(&ctx(""));
    assert!(diags.is_empty());
}
