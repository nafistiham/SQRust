use sqrust_core::FileContext;
use sqrust_rules::layout::comparison_operator_spacing::ComparisonOperatorSpacing;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(src: &str) -> Vec<sqrust_core::Diagnostic> {
    ComparisonOperatorSpacing.check(&ctx(src))
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        ComparisonOperatorSpacing.name(),
        "Layout/ComparisonOperatorSpacing"
    );
}

// ── No violation cases ────────────────────────────────────────────────────────

#[test]
fn spaced_less_than_no_violation() {
    let diags = check("WHERE a < b");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn spaced_greater_than_no_violation() {
    let diags = check("WHERE a > b");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn spaced_not_equal_no_violation() {
    let diags = check("WHERE a <> b");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn spaced_lte_no_violation() {
    let diags = check("WHERE a <= b");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn spaced_gte_no_violation() {
    let diags = check("WHERE a >= b");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn spaced_not_equal_bang_no_violation() {
    let diags = check("WHERE a != b");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn in_string_not_flagged() {
    // '<' inside string literal must not be flagged
    let diags = check("WHERE note = 'a<b'");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn in_comment_not_flagged() {
    // '<' inside line comment must not be flagged
    let diags = check("-- a<b");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

// ── Violation cases ───────────────────────────────────────────────────────────

#[test]
fn no_space_before_lt_flagged() {
    let diags = check("WHERE a<b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'a<b', got {}", diags.len());
}

#[test]
fn no_space_after_lt_flagged() {
    let diags = check("WHERE a <b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'a <b', got {}", diags.len());
}

#[test]
fn no_space_gt_flagged() {
    let diags = check("WHERE a>b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'a>b', got {}", diags.len());
}

#[test]
fn no_space_ne_flagged() {
    let diags = check("WHERE a<>b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'a<>b', got {}", diags.len());
}

#[test]
fn no_space_lte_flagged() {
    let diags = check("WHERE a<=b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'a<=b', got {}", diags.len());
}

// ── Message & position ────────────────────────────────────────────────────────

#[test]
fn message_contains_operator() {
    let diags = check("WHERE a<b");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('<'),
        "message should contain the operator '<', got: {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let diags = check("WHERE a<b");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1, got {}", diags[0].line);
}

#[test]
fn col_nonzero() {
    let diags = check("WHERE a<b");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1, "col must be >= 1, got {}", diags[0].col);
}
