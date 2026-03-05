use sqrust_core::FileContext;
use sqrust_rules::layout::max_blank_lines::MaxBlankLines;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(MaxBlankLines::default().name(), "MaxBlankLines");
}

#[test]
fn default_max_blank_lines_is_one() {
    assert_eq!(MaxBlankLines::default().max_blank_lines, 1);
}

// ── No violations ────────────────────────────────────────────────────────────

#[test]
fn parse_error_produces_no_violations() {
    let diags = MaxBlankLines::default().check(&ctx("SELECT FROM FROM"));
    assert!(diags.is_empty());
}

#[test]
fn no_blank_lines_produces_no_violations() {
    let diags = MaxBlankLines::default().check(&ctx("SELECT 1;\nSELECT 2;\n"));
    assert!(diags.is_empty());
}

#[test]
fn exactly_one_blank_line_produces_no_violations() {
    let diags = MaxBlankLines::default().check(&ctx("SELECT 1;\n\nSELECT 2;\n"));
    assert!(diags.is_empty());
}

#[test]
fn custom_max_two_with_two_blank_lines_produces_no_violations() {
    let rule = MaxBlankLines { max_blank_lines: 2 };
    let diags = rule.check(&ctx("SELECT 1;\n\n\nSELECT 2;\n"));
    assert!(diags.is_empty());
}

// ── Violations ───────────────────────────────────────────────────────────────

#[test]
fn two_consecutive_blank_lines_produces_one_violation() {
    let diags = MaxBlankLines::default().check(&ctx("SELECT 1;\n\n\nSELECT 2;\n"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_consecutive_blank_lines_produces_one_violation() {
    let diags = MaxBlankLines::default().check(&ctx("SELECT 1;\n\n\n\nSELECT 2;\n"));
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_separate_runs_produce_two_violations() {
    let diags =
        MaxBlankLines::default().check(&ctx("SELECT 1;\n\n\nSELECT 2;\n\n\nSELECT 3;\n"));
    assert_eq!(diags.len(), 2);
}

#[test]
fn custom_max_two_with_three_blank_lines_produces_one_violation() {
    let rule = MaxBlankLines { max_blank_lines: 2 };
    let diags = rule.check(&ctx("SELECT 1;\n\n\n\nSELECT 2;\n"));
    assert_eq!(diags.len(), 1);
}

// ── Line/col position ────────────────────────────────────────────────────────

#[test]
fn violation_line_points_to_second_blank_line() {
    // line 1: "SELECT 1;"
    // line 2: ""  ← first blank (ok)
    // line 3: ""  ← second blank → violation reported here
    // line 4: "SELECT 2;"
    let diags = MaxBlankLines::default().check(&ctx("SELECT 1;\n\n\nSELECT 2;\n"));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 3);
}

// ── Messages ─────────────────────────────────────────────────────────────────

#[test]
fn message_includes_actual_count_and_max() {
    // 2 consecutive blank lines with default max=1 → "2 found, maximum is 1"
    let diags = MaxBlankLines::default().check(&ctx("SELECT 1;\n\n\nSELECT 2;\n"));
    assert_eq!(
        diags[0].message,
        "Too many consecutive blank lines (2 found, maximum is 1)"
    );
}

// ── Fix ───────────────────────────────────────────────────────────────────────

#[test]
fn fix_collapses_three_blank_lines_to_one() {
    let c = ctx("SELECT 1;\n\n\n\nSELECT 2;\n");
    let fixed = MaxBlankLines::default().fix(&c).expect("fix should return Some");
    assert_eq!(fixed, "SELECT 1;\n\nSELECT 2;\n");
}

#[test]
fn fix_leaves_single_blank_lines_unchanged() {
    let c = ctx("SELECT 1;\n\nSELECT 2;\n");
    // No violation, fix should return None (nothing to fix)
    let result = MaxBlankLines::default().fix(&c);
    assert!(result.is_none());
}

#[test]
fn fix_handles_multiple_runs() {
    let c = ctx("SELECT 1;\n\n\nSELECT 2;\n\n\nSELECT 3;\n");
    let fixed = MaxBlankLines::default().fix(&c).expect("fix should return Some");
    assert_eq!(fixed, "SELECT 1;\n\nSELECT 2;\n\nSELECT 3;\n");
}
