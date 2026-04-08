use sqrust_core::FileContext;
use sqrust_rules::layout::space_around_bitwise_operator::SpaceAroundBitwiseOperator;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(src: &str) -> Vec<sqrust_core::Diagnostic> {
    SpaceAroundBitwiseOperator.check(&ctx(src))
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SpaceAroundBitwiseOperator.name(), "Layout/SpaceAroundBitwiseOperator");
}

// ── Violation cases ───────────────────────────────────────────────────────────

#[test]
fn ampersand_no_space_violation() {
    let diags = check("a&b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'a&b', got {}", diags.len());
}

#[test]
fn pipe_no_space_violation() {
    let diags = check("a|b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'a|b', got {}", diags.len());
}

#[test]
fn caret_no_space_violation() {
    let diags = check("a^b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'a^b', got {}", diags.len());
}

#[test]
fn tilde_no_space_violation() {
    // ~a with no preceding space (tilde directly adjacent to word char after it)
    let diags = check("x~b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'x~b', got {}", diags.len());
}

#[test]
fn select_bitwise_violation() {
    let diags = check("SELECT a&b FROM t");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'SELECT a&b FROM t', got {}", diags.len());
}

#[test]
fn multiple_operators_violation() {
    let diags = check("a&b, c|d");
    assert_eq!(diags.len(), 2, "expected 2 violations for 'a&b, c|d', got {}", diags.len());
}

// ── No-violation cases ────────────────────────────────────────────────────────

#[test]
fn ampersand_with_spaces_no_violation() {
    let diags = check("a & b");
    assert!(diags.is_empty(), "expected 0 violations for 'a & b', got {}", diags.len());
}

#[test]
fn pipe_with_spaces_no_violation() {
    let diags = check("a | b");
    assert!(diags.is_empty(), "expected 0 violations for 'a | b', got {}", diags.len());
}

#[test]
fn caret_with_spaces_no_violation() {
    let diags = check("a ^ b");
    assert!(diags.is_empty(), "expected 0 violations for 'a ^ b', got {}", diags.len());
}

#[test]
fn select_bitwise_no_violation() {
    let diags = check("SELECT a & b FROM t");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn bitwise_in_string_no_violation() {
    let diags = check("SELECT 'a&b'");
    assert!(diags.is_empty(), "expected 0 violations (& inside string), got {}", diags.len());
}

#[test]
fn bitwise_in_comment_no_violation() {
    let diags = check("SELECT 1 -- a&b\n");
    assert!(diags.is_empty(), "expected 0 violations (& inside comment), got {}", diags.len());
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty(), "expected 0 violations for empty file, got {}", diags.len());
}

// ── Message content ───────────────────────────────────────────────────────────

#[test]
fn message_contains_operator() {
    let diags = check("a&b");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('&'),
        "message should contain '&', got: {}",
        diags[0].message
    );
}
