use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::duplicate_group_by_column::DuplicateGroupByColumn;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    DuplicateGroupByColumn.check(&c)
}

// ── rule name ────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(DuplicateGroupByColumn.name(), "Structure/DuplicateGroupByColumn");
}

// ── violations ───────────────────────────────────────────────────────────────

#[test]
fn simple_duplicate_violation() {
    let diags = check("SELECT a, COUNT(*) FROM t GROUP BY a, b, a");
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_insensitive_duplicate_violation() {
    let diags = check("SELECT A FROM t GROUP BY A, a");
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_columns_one_duplicate_violation() {
    let diags = check("SELECT a, b, c FROM t GROUP BY a, b, c, b");
    assert_eq!(diags.len(), 1);
}

#[test]
fn violation_message_contains_column_name() {
    let diags = check("SELECT a, COUNT(*) FROM t GROUP BY a, b, a");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains('a'), "message should mention 'a': {}", diags[0].message);
}

#[test]
fn group_by_expression_duplicate_violation() {
    // Same expressions repeated — should be flagged
    let diags = check("SELECT YEAR(d) FROM t GROUP BY YEAR(d), YEAR(d)");
    assert_eq!(diags.len(), 1);
}

// ── no violations ────────────────────────────────────────────────────────────

#[test]
fn no_duplicate_no_violation() {
    let diags = check("SELECT a, b FROM t GROUP BY a, b");
    assert!(diags.is_empty());
}

#[test]
fn single_column_no_violation() {
    let diags = check("SELECT a FROM t GROUP BY a");
    assert!(diags.is_empty());
}

#[test]
fn no_group_by_no_violation() {
    let diags = check("SELECT COUNT(*) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn group_by_1_no_violation() {
    // Positional GROUP BY — no duplicate
    let diags = check("SELECT a FROM t GROUP BY 1");
    assert!(diags.is_empty());
}

#[test]
fn two_tables_no_violation() {
    let diags = check(
        "SELECT a, COUNT(*) FROM t1 GROUP BY a; SELECT b, COUNT(*) FROM t2 GROUP BY b",
    );
    assert!(diags.is_empty());
}

#[test]
fn group_by_expression_no_violation() {
    // Different function calls — no duplicate
    let diags = check("SELECT YEAR(d), MONTH(d) FROM t GROUP BY YEAR(d), MONTH(d)");
    assert!(diags.is_empty());
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_no_violation() {
    let c = ctx("SELECTT INVALID GARBAGE @@##");
    if !c.parse_errors.is_empty() {
        let diags = DuplicateGroupByColumn.check(&c);
        assert!(diags.is_empty());
    }
}

// ── empty file ────────────────────────────────────────────────────────────────

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}
