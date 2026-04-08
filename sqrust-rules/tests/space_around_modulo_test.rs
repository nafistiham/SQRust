use sqrust_core::FileContext;
use sqrust_rules::layout::space_around_modulo::SpaceAroundModulo;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(src: &str) -> Vec<sqrust_core::Diagnostic> {
    SpaceAroundModulo.check(&ctx(src))
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SpaceAroundModulo.name(), "Layout/SpaceAroundModulo");
}

// ── Violation cases ───────────────────────────────────────────────────────────

#[test]
fn no_spaces_violation() {
    let diags = check("a%b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'a%b', got {}", diags.len());
}

#[test]
fn space_before_only_violation() {
    let diags = check("a %b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'a %b', got {}", diags.len());
}

#[test]
fn space_after_only_violation() {
    let diags = check("a% b");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'a% b', got {}", diags.len());
}

#[test]
fn select_modulo_violation() {
    let diags = check("SELECT a%2 FROM t");
    assert_eq!(diags.len(), 1, "expected 1 violation for 'SELECT a%2 FROM t', got {}", diags.len());
}

#[test]
fn multiple_violations() {
    let diags = check("a%b, c%d");
    assert_eq!(diags.len(), 2, "expected 2 violations for 'a%b, c%d', got {}", diags.len());
}

// ── No-violation cases ────────────────────────────────────────────────────────

#[test]
fn both_spaces_no_violation() {
    let diags = check("a % b");
    assert!(diags.is_empty(), "expected 0 violations for 'a % b', got {}", diags.len());
}

#[test]
fn select_modulo_no_violation() {
    let diags = check("SELECT a % 2 FROM t");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn modulo_in_string_no_violation() {
    let diags = check("SELECT '100%'");
    assert!(diags.is_empty(), "expected 0 violations (% inside string), got {}", diags.len());
}

#[test]
fn modulo_in_comment_no_violation() {
    let diags = check("SELECT 1 -- 100%\n");
    assert!(diags.is_empty(), "expected 0 violations (% inside comment), got {}", diags.len());
}

#[test]
fn modulo_in_block_comment_no_violation() {
    let diags = check("SELECT 1 /* 100% done */");
    assert!(diags.is_empty(), "expected 0 violations (% inside block comment), got {}", diags.len());
}

#[test]
fn like_wildcard_in_string_no_violation() {
    let diags = check("WHERE col LIKE 'foo%'");
    assert!(diags.is_empty(), "expected 0 violations (LIKE wildcard in string), got {}", diags.len());
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty(), "expected 0 violations for empty file, got {}", diags.len());
}

#[test]
fn newline_adjacent_no_violation() {
    // % at end of line followed by newline — no adjacent word character
    let diags = check("SELECT 1\n-- 50%\nFROM t");
    assert!(diags.is_empty(), "expected 0 violations (% in comment with newline), got {}", diags.len());
}

// ── Message content ───────────────────────────────────────────────────────────

#[test]
fn message_mentions_modulo() {
    let diags = check("a%b");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('%'),
        "message should mention '%', got: {}",
        diags[0].message
    );
}
