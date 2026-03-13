use sqrust_core::FileContext;
use sqrust_rules::layout::function_call_spacing::FunctionCallSpacing;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(FunctionCallSpacing.name(), "Layout/FunctionCallSpacing");
}

// ── Basic violations ──────────────────────────────────────────────────────────

#[test]
fn count_with_space_before_paren_produces_one_violation() {
    let diags = FunctionCallSpacing.check(&ctx("SELECT COUNT (*) FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn count_without_space_produces_no_violations() {
    let diags = FunctionCallSpacing.check(&ctx("SELECT COUNT(*) FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn coalesce_with_space_before_paren_produces_one_violation() {
    let diags = FunctionCallSpacing.check(&ctx("SELECT COALESCE (a, b) FROM t"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn coalesce_without_space_produces_no_violations() {
    let diags = FunctionCallSpacing.check(&ctx("SELECT COALESCE(a, b) FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn max_with_space_before_paren_produces_one_violation() {
    let diags = FunctionCallSpacing.check(&ctx("SELECT MAX (col) FROM t"));
    assert_eq!(diags.len(), 1);
}

// ── Keyword exclusions ────────────────────────────────────────────────────────

#[test]
fn in_keyword_before_paren_produces_no_violations() {
    let diags = FunctionCallSpacing.check(&ctx("WHERE id IN (1, 2)"));
    assert!(diags.is_empty());
}

#[test]
fn exists_keyword_before_paren_produces_no_violations() {
    let diags = FunctionCallSpacing.check(&ctx("WHERE EXISTS (SELECT 1 FROM t)"));
    assert!(diags.is_empty());
}

#[test]
fn over_keyword_before_paren_produces_no_violations() {
    let diags =
        FunctionCallSpacing.check(&ctx("SELECT SUM(val) OVER (PARTITION BY grp) FROM t"));
    assert!(diags.is_empty());
}

#[test]
fn as_keyword_before_paren_produces_no_violations() {
    let diags = FunctionCallSpacing.check(&ctx("WITH c AS (SELECT 1) SELECT * FROM c"));
    assert!(diags.is_empty());
}

#[test]
fn not_keyword_before_paren_produces_no_violations() {
    let diags = FunctionCallSpacing.check(&ctx("SELECT NOT (col = 1) FROM t"));
    assert!(diags.is_empty());
}

// ── More function names ───────────────────────────────────────────────────────

#[test]
fn lower_with_space_produces_one_violation() {
    let diags = FunctionCallSpacing.check(&ctx("SELECT LOWER (col) FROM t"));
    assert_eq!(diags.len(), 1);
}

// ── Multiple violations ───────────────────────────────────────────────────────

#[test]
fn multiple_violations_are_all_reported() {
    let diags =
        FunctionCallSpacing.check(&ctx("SELECT MAX (a), MIN (b) FROM t"));
    assert_eq!(diags.len(), 2);
}

// ── String literals ───────────────────────────────────────────────────────────

#[test]
fn pattern_inside_string_produces_no_violations() {
    let diags = FunctionCallSpacing.check(&ctx("SELECT 'COUNT (*)' FROM t"));
    assert!(diags.is_empty());
}

// ── Parse error resilience ────────────────────────────────────────────────────

#[test]
fn parse_error_source_still_runs() {
    // Malformed SQL but source-level check should still execute.
    let diags = FunctionCallSpacing.check(&ctx("SELECT COUNT (*) FROM FROM"));
    assert_eq!(diags.len(), 1);
}

// ── Message text ──────────────────────────────────────────────────────────────

#[test]
fn violation_message_contains_function_name_and_hint() {
    let diags = FunctionCallSpacing.check(&ctx("SELECT COUNT (*) FROM t"));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("COUNT"),
        "message should contain the function name"
    );
    assert!(
        diags[0].message.contains("COUNT("),
        "message should contain the corrected form"
    );
}
