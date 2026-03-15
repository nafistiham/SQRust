use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::cast_without_length::CastWithoutLength;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    CastWithoutLength.check(&c)
}

// ── rule name ────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(CastWithoutLength.name(), "Ambiguous/CastWithoutLength");
}

// ── clean cases (no violation) ────────────────────────────────────────────────

#[test]
fn cast_varchar_with_length_no_violation() {
    let diags = check("SELECT CAST(col AS VARCHAR(100)) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn cast_char_with_length_no_violation() {
    let diags = check("SELECT CAST(col AS CHAR(10)) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn cast_nvarchar_with_length_no_violation() {
    let diags = check("SELECT CAST(col AS NVARCHAR(200)) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn cast_nchar_with_length_no_violation() {
    let diags = check("SELECT CAST(col AS NCHAR(5)) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn cast_integer_no_violation() {
    let diags = check("SELECT CAST(col AS INTEGER) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_cast_at_all_no_violation() {
    let diags = check("SELECT col1, col2 FROM t WHERE col1 = 1");
    assert!(diags.is_empty());
}

// ── violations ───────────────────────────────────────────────────────────────

#[test]
fn cast_varchar_without_length_one_violation() {
    let diags = check("SELECT CAST(col AS VARCHAR) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cast_char_without_length_one_violation() {
    let diags = check("SELECT CAST(col AS CHAR) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cast_nvarchar_without_length_one_violation() {
    let diags = check("SELECT CAST(col AS NVARCHAR) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cast_nchar_without_length_one_violation() {
    let diags = check("SELECT CAST(col AS NCHAR) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── case-insensitivity ───────────────────────────────────────────────────────

#[test]
fn lowercase_cast_varchar_without_length_one_violation() {
    let diags = check("SELECT cast(col as varchar) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn mixed_case_cast_varchar_without_length_one_violation() {
    let diags = check("SELECT Cast(col As Varchar) FROM t");
    assert_eq!(diags.len(), 1);
}

// ── message content ───────────────────────────────────────────────────────────

#[test]
fn violation_message_mentions_type_and_length_hint() {
    let diags = check("SELECT CAST(col AS VARCHAR) FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_lowercase().contains("varchar"),
        "message should mention the type"
    );
    assert!(
        diags[0].message.contains("(N)") || diags[0].message.contains("length"),
        "message should mention length"
    );
}

// ── diagnostic rule field ──────────────────────────────────────────────────────

#[test]
fn diagnostic_rule_field_is_correct() {
    let diags = check("SELECT CAST(col AS VARCHAR) FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/CastWithoutLength");
}

// ── multiple violations ───────────────────────────────────────────────────────

#[test]
fn two_casts_without_length_two_violations() {
    let diags = check("SELECT CAST(a AS VARCHAR), CAST(b AS NCHAR) FROM t");
    assert_eq!(diags.len(), 2);
}

// ── skip string literals ─────────────────────────────────────────────────────

#[test]
fn varchar_inside_string_literal_no_violation() {
    // The pattern "AS VARCHAR)" appears inside a string — should not flag.
    let diags = check("SELECT 'CAST(col AS VARCHAR)' FROM t");
    assert!(diags.is_empty());
}

// ── skip comments ────────────────────────────────────────────────────────────

#[test]
fn varchar_inside_line_comment_no_violation() {
    let diags = check("SELECT col FROM t -- CAST(col AS VARCHAR)");
    assert!(diags.is_empty());
}
