use sqrust_core::{FileContext, Rule};
use sqrust_rules::structure::deeply_nested_case::DeeplyNestedCase;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    DeeplyNestedCase::default().check(&ctx(sql))
}

fn check_with(sql: &str, max_depth: usize) -> Vec<sqrust_core::Diagnostic> {
    DeeplyNestedCase { max_depth }.check(&ctx(sql))
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(DeeplyNestedCase::default().name(), "Structure/DeeplyNestedCase");
}

// ── no CASE — no violation ────────────────────────────────────────────────────

#[test]
fn no_case_no_violation() {
    let diags = check("SELECT * FROM t");
    assert!(diags.is_empty());
}

// ── depth 1 — no violation ────────────────────────────────────────────────────

#[test]
fn one_level_no_violation() {
    let diags = check("SELECT CASE WHEN x=1 THEN 'a' END FROM t");
    assert!(diags.is_empty());
}

// ── depth 2 — no violation (max is 3) ────────────────────────────────────────

#[test]
fn two_levels_no_violation() {
    let sql = "SELECT CASE WHEN x=1 THEN CASE WHEN y=1 THEN 'a' ELSE 'b' END ELSE 'c' END FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

// ── depth 3 == max — still ok (not strictly greater) ─────────────────────────

#[test]
fn three_levels_no_violation() {
    let sql = "SELECT CASE WHEN a=1 THEN \
               CASE WHEN b=1 THEN \
               CASE WHEN c=1 THEN 'x' ELSE 'y' END \
               ELSE 'z' END \
               ELSE 'w' END FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

// ── depth 4 > max 3 — violation ───────────────────────────────────────────────

#[test]
fn four_levels_violation() {
    let sql = "SELECT CASE WHEN a=1 THEN \
               CASE WHEN b=1 THEN \
               CASE WHEN c=1 THEN \
               CASE WHEN d=1 THEN 'x' END \
               END END ELSE 'z' END FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

// ── CASE in string — no violation ─────────────────────────────────────────────

#[test]
fn case_in_string_no_violation() {
    let diags = check("SELECT 'CASE CASE CASE CASE END END END END' FROM t");
    assert!(diags.is_empty());
}

// ── CASE in comment — no violation ────────────────────────────────────────────

#[test]
fn case_in_comment_no_violation() {
    let diags = check("-- CASE CASE CASE CASE\nSELECT 1");
    assert!(diags.is_empty());
}

// ── diagnostic message contains depth ────────────────────────────────────────

#[test]
fn violation_message_contains_depth() {
    let sql = "SELECT CASE WHEN a=1 THEN \
               CASE WHEN b=1 THEN \
               CASE WHEN c=1 THEN \
               CASE WHEN d=1 THEN 'x' END \
               END END ELSE 'z' END FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('4'),
        "message should contain depth 4; got: {}",
        diags[0].message
    );
}

// ── diagnostic message contains max ──────────────────────────────────────────

#[test]
fn violation_message_contains_max() {
    let sql = "SELECT CASE WHEN a=1 THEN \
               CASE WHEN b=1 THEN \
               CASE WHEN c=1 THEN \
               CASE WHEN d=1 THEN 'x' END \
               END END ELSE 'z' END FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('3'),
        "message should contain max 3; got: {}",
        diags[0].message
    );
}

// ── END outside CASE context does not decrement below zero ───────────────────

#[test]
fn end_keyword_not_in_case_context_no_violation() {
    // BEGIN/END in non-CASE context: the rule tracks END only when depth > 0,
    // so this should not cause a spurious violation or panic.
    let diags = check("BEGIN SELECT 1 END");
    assert!(diags.is_empty());
}

// ── two separate non-nested CASE blocks — no violation ────────────────────────

#[test]
fn multiple_separate_cases_no_violation() {
    let sql = "SELECT \
               CASE WHEN x=1 THEN 'a' ELSE 'b' END, \
               CASE WHEN y=2 THEN 'c' ELSE 'd' END \
               FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

// ── empty file — no violation ─────────────────────────────────────────────────

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

// ── custom max_depth ──────────────────────────────────────────────────────────

#[test]
fn custom_max_depth_5_depth_6_violation() {
    // Sanity check that configurable max works.
    let sql = "SELECT CASE WHEN a=1 THEN \
               CASE WHEN b=1 THEN \
               CASE WHEN c=1 THEN \
               CASE WHEN d=1 THEN \
               CASE WHEN e=1 THEN \
               CASE WHEN f=1 THEN 'x' END \
               END END END END ELSE 'z' END FROM t";
    let diags = check_with(sql, 5);
    assert_eq!(diags.len(), 1);
}
