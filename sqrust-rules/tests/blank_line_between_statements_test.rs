use sqrust_core::FileContext;
use sqrust_rules::layout::blank_line_between_statements::BlankLineBetweenStatements;
use sqrust_core::Rule;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(src: &str) -> Vec<sqrust_core::Diagnostic> {
    BlankLineBetweenStatements.check(&ctx(src))
}

// ── Rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(BlankLineBetweenStatements.name(), "Layout/BlankLineBetweenStatements");
}

// ── No violation cases ────────────────────────────────────────────────────────

#[test]
fn single_statement_no_violation() {
    let diags = check("SELECT 1;");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn two_statements_with_blank_line_no_violation() {
    let diags = check("SELECT 1;\n\nSELECT 2;");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn three_statements_all_separated_no_violation() {
    let diags = check("SELECT 1;\n\nSELECT 2;\n\nSELECT 3;");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn trailing_newlines_no_violation() {
    // Trailing newlines after last statement — not a new statement, no flag
    let diags = check("SELECT 1;\n\n");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn no_semicolons_no_violation() {
    // No semicolons means no statement boundaries detected
    let diags = check("SELECT 1");
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

#[test]
fn multiline_statement_with_blank_line_no_violation() {
    // Multi-line SELECT separated by blank line from the next statement
    let src = "SELECT id,\n       name\nFROM users;\n\nSELECT 2;";
    let diags = check(src);
    assert!(diags.is_empty(), "expected 0 violations, got {}", diags.len());
}

// ── Violation cases ───────────────────────────────────────────────────────────

#[test]
fn two_statements_without_blank_line_one_violation() {
    let diags = check("SELECT 1;\nSELECT 2;");
    assert_eq!(diags.len(), 1, "expected 1 violation, got {}", diags.len());
}

#[test]
fn three_statements_none_separated_two_violations() {
    let diags = check("SELECT 1;\nSELECT 2;\nSELECT 3;");
    assert_eq!(diags.len(), 2, "expected 2 violations, got {}", diags.len());
}

#[test]
fn three_statements_first_missing_blank_one_violation() {
    // First pair has no blank, second pair has blank → 1 violation
    let diags = check("SELECT 1;\nSELECT 2;\n\nSELECT 3;");
    assert_eq!(diags.len(), 1, "expected 1 violation, got {}", diags.len());
}

// ── Message & position ────────────────────────────────────────────────────────

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT 1;\nSELECT 2;");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_lowercase().contains("blank")
            || diags[0].message.to_lowercase().contains("statement")
            || diags[0].message.to_lowercase().contains("separated"),
        "message should mention blank/statement/separated, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_is_start_of_second_statement() {
    // "SELECT 1;\nSELECT 2;" — violation col must be 1 (start of the line)
    let diags = check("SELECT 1;\nSELECT 2;");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 1, "col should be 1 (start of statement), got {}", diags[0].col);
}

#[test]
fn violation_at_correct_line() {
    // "SELECT 1;\nSELECT 2;" — second statement is on line 2
    let diags = check("SELECT 1;\nSELECT 2;");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2, "expected violation at line 2, got {}", diags[0].line);
}
