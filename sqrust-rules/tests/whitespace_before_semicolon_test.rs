use sqrust_core::FileContext;
use sqrust_rules::layout::whitespace_before_semicolon::WhitespaceBeforeSemicolon;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    WhitespaceBeforeSemicolon.check(&ctx(sql))
}

// ── Rule metadata ──────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(WhitespaceBeforeSemicolon.name(), "Layout/WhitespaceBeforeSemicolon");
}

// ── No violations ─────────────────────────────────────────────────────────────

#[test]
fn no_whitespace_before_semicolon_no_violation() {
    let diags = check("SELECT 1;");
    assert!(diags.is_empty());
}

#[test]
fn no_semicolons_no_violation() {
    let diags = check("SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn space_in_string_not_flagged() {
    // The ' ;' is inside a string literal — not a real semicolon
    let diags = check("SELECT 'hello ;' FROM t;");
    assert!(diags.is_empty());
}

// ── Violations ────────────────────────────────────────────────────────────────

#[test]
fn space_before_semicolon_one_violation() {
    let diags = check("SELECT 1 ;");
    assert_eq!(diags.len(), 1);
}

#[test]
fn tab_before_semicolon_one_violation() {
    let diags = check("SELECT 1\t;");
    assert_eq!(diags.len(), 1);
}

#[test]
fn multiple_spaces_before_semicolon_one_violation() {
    // Multiple spaces still counts as one violation per semicolon
    let diags = check("SELECT 1   ;");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_statements_both_bad_two_violations() {
    let diags = check("SELECT 1 ;\nSELECT 2 ;");
    assert_eq!(diags.len(), 2);
}

// ── Message and position ──────────────────────────────────────────────────────

#[test]
fn violation_message_mentions_whitespace_or_semicolon() {
    let diags = check("SELECT 1 ;");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.to_lowercase().contains("whitespace") || msg.to_lowercase().contains("semicolon"),
        "message should mention 'whitespace' or 'semicolon', got: {msg}"
    );
}

#[test]
fn violation_line_is_nonzero() {
    let diags = check("SELECT 1 ;");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn violation_col_is_nonzero() {
    let diags = check("SELECT 1 ;");
    assert_eq!(diags.len(), 1);
    // col should point to the whitespace before ';', which is at position 9 (1-indexed)
    assert!(diags[0].col >= 1);
}

// ── Fix support ───────────────────────────────────────────────────────────────

#[test]
fn fix_removes_space_before_semicolon() {
    let c = ctx("SELECT 1 ;");
    let fixed = WhitespaceBeforeSemicolon.fix(&c).expect("fix should be available");
    assert_eq!(fixed, "SELECT 1;");
}

#[test]
fn fix_removes_tab_before_semicolon() {
    let c = ctx("SELECT 1\t;");
    let fixed = WhitespaceBeforeSemicolon.fix(&c).expect("fix should be available");
    assert_eq!(fixed, "SELECT 1;");
}

#[test]
fn fix_multiple_statements() {
    let c = ctx("SELECT 1 ;\nSELECT 2   ;");
    let fixed = WhitespaceBeforeSemicolon.fix(&c).expect("fix should be available");
    assert_eq!(fixed, "SELECT 1;\nSELECT 2;");
}
