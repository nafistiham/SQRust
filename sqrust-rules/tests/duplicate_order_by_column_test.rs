use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::duplicate_order_by_column::DuplicateOrderByColumn;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    DuplicateOrderByColumn.check(&c)
}

// ── rule name ────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(DuplicateOrderByColumn.name(), "Structure/DuplicateOrderByColumn");
}

// ── violations ───────────────────────────────────────────────────────────────

#[test]
fn simple_duplicate_violation() {
    let diags = check("SELECT a, b FROM t ORDER BY a, b, a");
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_insensitive_duplicate_violation() {
    let diags = check("SELECT a FROM t ORDER BY A, a");
    assert_eq!(diags.len(), 1);
}

#[test]
fn duplicate_with_direction_violation() {
    // a appears twice regardless of ASC/DESC
    let diags = check("SELECT a FROM t ORDER BY a ASC, b, a DESC");
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_columns_two_duplicate_violation() {
    let diags = check("SELECT a, b, c FROM t ORDER BY a, b, c, a");
    assert_eq!(diags.len(), 1);
}

#[test]
fn violation_message_contains_column_name() {
    let diags = check("SELECT a, b FROM t ORDER BY a, b, a");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains('a'), "message should mention 'a': {}", diags[0].message);
}

// ── no violations ────────────────────────────────────────────────────────────

#[test]
fn no_duplicate_no_violation() {
    let diags = check("SELECT a, b FROM t ORDER BY a, b");
    assert!(diags.is_empty());
}

#[test]
fn single_column_no_violation() {
    let diags = check("SELECT a FROM t ORDER BY a");
    assert!(diags.is_empty());
}

#[test]
fn no_order_by_no_violation() {
    let diags = check("SELECT a FROM t");
    assert!(diags.is_empty());
}

#[test]
fn two_separate_queries_each_no_violation() {
    let diags = check("SELECT a, b FROM t1 ORDER BY a, b; SELECT c, d FROM t2 ORDER BY c, d");
    assert!(diags.is_empty());
}

#[test]
fn order_by_expr_no_violation() {
    // Different expressions — no duplicates
    let diags = check("SELECT a FROM t ORDER BY a + 1, b");
    assert!(diags.is_empty());
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_no_violation() {
    let c = ctx("SELECTT INVALID GARBAGE @@##");
    if !c.parse_errors.is_empty() {
        let diags = DuplicateOrderByColumn.check(&c);
        assert!(diags.is_empty());
    }
}

// ── empty file ────────────────────────────────────────────────────────────────

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}
