use sqrust_core::FileContext;
use sqrust_rules::layout::indentation_consistency::IndentationConsistency;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(IndentationConsistency.name(), "Layout/IndentationConsistency");
}

// ── Empty / no indentation ────────────────────────────────────────────────────

#[test]
fn empty_file_produces_no_violations() {
    let diags = IndentationConsistency.check(&ctx(""));
    assert!(diags.is_empty());
}

#[test]
fn file_with_no_indentation_produces_no_violations() {
    let diags = IndentationConsistency.check(&ctx("SELECT a, b\nFROM t\nWHERE a = 1"));
    assert!(diags.is_empty());
}

// ── Consistent indentation ────────────────────────────────────────────────────

#[test]
fn consistent_two_space_indentation_produces_no_violations() {
    let src = "SELECT\n  a,\n  b\nFROM t";
    let diags = IndentationConsistency.check(&ctx(src));
    assert!(diags.is_empty());
}

#[test]
fn consistent_four_space_indentation_produces_no_violations() {
    let src = "SELECT\n    a,\n    b\nFROM t";
    let diags = IndentationConsistency.check(&ctx(src));
    assert!(diags.is_empty());
}

#[test]
fn two_and_four_space_lines_gcd_two_produces_no_violations() {
    // 2 spaces at depth 1, 4 spaces at depth 2 — consistent 2-space style
    let src = "SELECT\n  a,\n    b\nFROM t";
    let diags = IndentationConsistency.check(&ctx(src));
    assert!(diags.is_empty());
}

#[test]
fn four_and_eight_space_lines_gcd_four_produces_no_violations() {
    let src = "SELECT\n    a,\n        b\nFROM t";
    let diags = IndentationConsistency.check(&ctx(src));
    assert!(diags.is_empty());
}

#[test]
fn two_four_six_space_lines_gcd_two_produces_no_violations() {
    let src = "SELECT\n  a,\n    b,\n      c\nFROM t";
    let diags = IndentationConsistency.check(&ctx(src));
    assert!(diags.is_empty());
}

// ── Inconsistent indentation ──────────────────────────────────────────────────

#[test]
fn one_space_indentation_gcd_one_produces_one_violation() {
    // Single-space indent — GCD of all counts is 1
    let src = "SELECT\n a,\n b\nFROM t";
    let diags = IndentationConsistency.check(&ctx(src));
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_two_and_three_space_indentation_gcd_one_produces_one_violation() {
    // 2-space and 3-space lines — GCD = 1
    let src = "SELECT\n  a,\n   b\nFROM t";
    let diags = IndentationConsistency.check(&ctx(src));
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_two_four_three_space_lines_gcd_one_produces_one_violation() {
    // 2, 4, 3 — GCD = 1
    let src = "SELECT\n  a,\n    b,\n   c\nFROM t";
    let diags = IndentationConsistency.check(&ctx(src));
    assert_eq!(diags.len(), 1);
}

// ── Only comments ─────────────────────────────────────────────────────────────

#[test]
fn file_with_only_comments_produces_no_violations() {
    // Comment lines are skipped; no indented code lines
    let src = "-- comment one\n-- comment two";
    let diags = IndentationConsistency.check(&ctx(src));
    assert!(diags.is_empty());
}

// ── Diagnostic placement ─────────────────────────────────────────────────────

#[test]
fn violation_is_reported_at_line_one_col_one() {
    let src = "SELECT\n a,\n b\nFROM t";
    let diags = IndentationConsistency.check(&ctx(src));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 1);
}

// ── Message text ──────────────────────────────────────────────────────────────

#[test]
fn violation_message_is_correct() {
    let src = "SELECT\n a\nFROM t";
    let diags = IndentationConsistency.check(&ctx(src));
    assert_eq!(
        diags[0].message,
        "Inconsistent indentation detected — lines use mixed indentation widths"
    );
}

// ── Only one diagnostic per file ─────────────────────────────────────────────

#[test]
fn at_most_one_violation_per_file() {
    // Many inconsistently indented lines — still one diagnostic
    let src = "SELECT\n a,\n  b,\n   c,\n    d\nFROM t";
    let diags = IndentationConsistency.check(&ctx(src));
    assert_eq!(diags.len(), 1);
}
