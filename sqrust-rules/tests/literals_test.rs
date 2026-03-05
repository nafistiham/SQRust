use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::capitalisation::literals::Literals;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    Literals.check(&ctx)
}

fn fix(sql: &str) -> Option<String> {
    let ctx = FileContext::from_source(sql, "test.sql");
    Literals.fix(&ctx)
}

// ─── rule meta ───────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(Literals.name(), "Capitalisation/Literals");
}

// ─── TRUE ────────────────────────────────────────────────────────────────────

#[test]
fn lowercase_true_flagged() {
    let diags = check("SELECT * FROM t WHERE active = true");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Capitalisation/Literals");
    assert_eq!(
        diags[0].message,
        "Literal 'true' should be 'TRUE'"
    );
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 32);
}

#[test]
fn mixed_case_true_flagged() {
    let diags = check("SELECT * FROM t WHERE active = True");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Literal 'True' should be 'TRUE'");
}

#[test]
fn uppercase_true_no_violation() {
    assert!(check("SELECT * FROM t WHERE active = TRUE").is_empty());
}

// ─── FALSE ───────────────────────────────────────────────────────────────────

#[test]
fn lowercase_false_flagged() {
    let diags = check("SELECT * FROM t WHERE val = false");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Literal 'false' should be 'FALSE'");
}

#[test]
fn uppercase_false_no_violation() {
    assert!(check("SELECT * FROM t WHERE val = FALSE").is_empty());
}

// ─── NULL ────────────────────────────────────────────────────────────────────

#[test]
fn lowercase_null_flagged() {
    let diags = check("SELECT * FROM t WHERE col IS null");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Literal 'null' should be 'NULL'");
}

#[test]
fn uppercase_null_no_violation() {
    assert!(check("SELECT * FROM t WHERE col IS NULL").is_empty());
}

// ─── skip zones ──────────────────────────────────────────────────────────────

#[test]
fn literal_inside_single_quoted_string_skipped() {
    assert!(check("SELECT * FROM t WHERE col = 'true'").is_empty());
}

#[test]
fn literal_inside_line_comment_skipped() {
    assert!(check("SELECT col FROM t -- WHERE col = null").is_empty());
}

#[test]
fn literal_inside_double_quoted_identifier_skipped() {
    assert!(check(r#"SELECT "null" FROM t"#).is_empty());
}

// ─── word-boundary guard ─────────────────────────────────────────────────────

#[test]
fn nullable_not_flagged() {
    // "nullable" starts with "null" but is a longer word — must not fire
    assert!(check("SELECT nullable FROM t").is_empty());
}

#[test]
fn nullability_not_flagged() {
    assert!(check("SELECT nullability FROM t").is_empty());
}

// ─── multiple violations ─────────────────────────────────────────────────────

#[test]
fn multiple_violations_on_same_line() {
    // "true" and "false" on the same line — expect 2 diagnostics
    let diags = check("SELECT * FROM t WHERE a = true AND b = false");
    assert_eq!(diags.len(), 2);
}

#[test]
fn mixed_violations_null_and_false() {
    let diags = check("SELECT * FROM t WHERE col IS null OR flag = false");
    assert_eq!(diags.len(), 2);
}

// ─── fix() ───────────────────────────────────────────────────────────────────

#[test]
fn fix_lowercase_true() {
    let fixed = fix("SELECT * FROM t WHERE active = true");
    assert_eq!(fixed, Some("SELECT * FROM t WHERE active = TRUE".to_string()));
}

#[test]
fn fix_mixed_literals() {
    let fixed = fix("SELECT * FROM t WHERE col IS null OR flag = false");
    assert_eq!(
        fixed,
        Some("SELECT * FROM t WHERE col IS NULL OR flag = FALSE".to_string())
    );
}

#[test]
fn fix_returns_none_when_no_violations() {
    assert_eq!(fix("SELECT * FROM t WHERE active = TRUE"), None);
}
