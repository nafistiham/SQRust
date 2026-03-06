use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::function_call_depth::FunctionCallDepth;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    FunctionCallDepth::default().check(&c)
}

fn check_with(sql: &str, max_depth: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    FunctionCallDepth { max_depth }.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(FunctionCallDepth::default().name(), "FunctionCallDepth");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM BROKEN CALL(()");
    assert!(diags.is_empty());
}

// ── default max_depth = 3 ─────────────────────────────────────────────────────

#[test]
fn default_max_depth_is_three() {
    assert_eq!(FunctionCallDepth::default().max_depth, 3);
}

// ── depth 1 → 0 violations ───────────────────────────────────────────────────

#[test]
fn depth_1_no_violation() {
    // UPPER(name) → depth 1
    let diags = check("SELECT UPPER(name) FROM t");
    assert!(diags.is_empty());
}

// ── depth 2 → 0 violations ───────────────────────────────────────────────────

#[test]
fn depth_2_no_violation() {
    // UPPER(LOWER(name)) → depth 2
    let diags = check("SELECT UPPER(LOWER(name)) FROM t");
    assert!(diags.is_empty());
}

// ── depth 3 (at max) → 0 violations ──────────────────────────────────────────

#[test]
fn depth_3_at_max_no_violation() {
    // UPPER(LOWER(COALESCE(name, ''))) → depth 3
    let diags = check("SELECT UPPER(LOWER(COALESCE(name, ''))) FROM t");
    assert!(diags.is_empty());
}

// ── depth 4 (over max 3) → 1 violation ───────────────────────────────────────

#[test]
fn depth_4_over_max_one_violation() {
    // ABS(UPPER(LOWER(COALESCE(name, '')))) → depth 4
    let diags = check("SELECT ABS(UPPER(LOWER(COALESCE(name, '')))) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── custom max 2, depth 3 → 1 violation ──────────────────────────────────────

#[test]
fn custom_max_2_depth_3_one_violation() {
    // UPPER(LOWER(COALESCE(name, ''))) → depth 3, max 2
    let diags = check_with("SELECT UPPER(LOWER(COALESCE(name, ''))) FROM t", 2);
    assert_eq!(diags.len(), 1);
}

// ── custom max 2, depth 2 → 0 violations ─────────────────────────────────────

#[test]
fn custom_max_2_depth_2_no_violation() {
    // UPPER(LOWER(name)) → depth 2, max 2
    let diags = check_with("SELECT UPPER(LOWER(name)) FROM t", 2);
    assert!(diags.is_empty());
}

// ── no function calls → 0 violations ─────────────────────────────────────────

#[test]
fn no_function_calls_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

// ── message contains depth and max ───────────────────────────────────────────

#[test]
fn message_contains_depth_and_max() {
    // ABS(UPPER(LOWER(COALESCE(name, '')))) → depth 4, default max 3
    let diags = check("SELECT ABS(UPPER(LOWER(COALESCE(name, '')))) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('4'),
        "message should contain the nesting depth (4): got: {}",
        diags[0].message
    );
    assert!(
        diags[0].message.contains('3'),
        "message should contain the max (3): got: {}",
        diags[0].message
    );
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT ABS(UPPER(LOWER(COALESCE(name, '')))) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── two independent deep nesting chains → 2 violations ───────────────────────

#[test]
fn two_deep_functions_two_violations() {
    // Two separate depth-4 nestings in the same SELECT
    let diags = check(
        "SELECT ABS(UPPER(LOWER(COALESCE(a, '')))), ABS(UPPER(LOWER(COALESCE(b, '')))) FROM t",
    );
    assert_eq!(diags.len(), 2);
}

// ── max 0, any function call → 1 violation ───────────────────────────────────

#[test]
fn depth_1_with_custom_max_0_one_violation() {
    // UPPER(name) → depth 1, max 0 → violation
    let diags = check_with("SELECT UPPER(name) FROM t", 0);
    assert_eq!(diags.len(), 1);
}
