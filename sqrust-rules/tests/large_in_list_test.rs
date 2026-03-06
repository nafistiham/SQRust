use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::large_in_list::LargeInList;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    LargeInList::default().check(&c)
}

fn check_with(sql: &str, max_values: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    LargeInList { max_values }.check(&c)
}

/// Build a SQL string with an IN list of `n` values.
fn make_in_list(n: usize) -> String {
    let values: Vec<String> = (1..=n).map(|i| i.to_string()).collect();
    format!("SELECT id FROM t WHERE id IN ({})", values.join(", "))
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(LargeInList::default().name(), "LargeInList");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM IN BROKEN WHERE");
    assert!(diags.is_empty());
}

// ── default max_values = 10 ───────────────────────────────────────────────────

#[test]
fn default_max_values_is_ten() {
    assert_eq!(LargeInList::default().max_values, 10);
}

// ── exactly at max (10 values) → 0 violations ─────────────────────────────────

#[test]
fn in_list_at_max_no_violation() {
    let diags = check(&make_in_list(10));
    assert!(diags.is_empty());
}

// ── 11 values (over default max) → 1 violation ────────────────────────────────

#[test]
fn in_list_over_max_one_violation() {
    let diags = check(&make_in_list(11));
    assert_eq!(diags.len(), 1);
}

// ── 5 values (under default max) → 0 violations ──────────────────────────────

#[test]
fn in_list_under_max_no_violation() {
    let diags = check(&make_in_list(5));
    assert!(diags.is_empty());
}

// ── custom max 3 with 4 values → 1 violation ─────────────────────────────────

#[test]
fn custom_max_3_with_4_values_one_violation() {
    let diags = check_with(&make_in_list(4), 3);
    assert_eq!(diags.len(), 1);
}

// ── custom max 3 with 3 values → 0 violations ────────────────────────────────

#[test]
fn custom_max_3_with_3_values_no_violation() {
    let diags = check_with(&make_in_list(3), 3);
    assert!(diags.is_empty());
}

// ── NOT IN with 11 values → 1 violation ──────────────────────────────────────

#[test]
fn not_in_large_list_one_violation() {
    let values: Vec<String> = (1..=11).map(|i| i.to_string()).collect();
    let sql = format!(
        "SELECT id FROM t WHERE id NOT IN ({})",
        values.join(", ")
    );
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
}

// ── IN subquery → 0 violations ───────────────────────────────────────────────

#[test]
fn in_subquery_no_violation() {
    let sql = "SELECT id FROM t WHERE id IN (SELECT id FROM other)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

// ── message contains count and max ───────────────────────────────────────────

#[test]
fn message_contains_count_and_max() {
    let diags = check(&make_in_list(11));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("11"),
        "message should contain the IN list count"
    );
    assert!(
        diags[0].message.contains("10"),
        "message should contain the max"
    );
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check(&make_in_list(11));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── two large IN lists → 2 violations ────────────────────────────────────────

#[test]
fn two_large_in_lists_two_violations() {
    let values: Vec<String> = (1..=11).map(|i| i.to_string()).collect();
    let list = values.join(", ");
    let sql = format!(
        "SELECT id FROM t WHERE id IN ({list}) OR id IN ({list})"
    );
    let diags = check(&sql);
    assert_eq!(diags.len(), 2);
}

// ── no IN expression → 0 violations ─────────────────────────────────────────

#[test]
fn no_in_expression_no_violation() {
    let diags = check("SELECT id, name FROM t WHERE id > 1");
    assert!(diags.is_empty());
}
