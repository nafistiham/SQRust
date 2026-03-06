use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::select_null_expression::SelectNullExpression;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    SelectNullExpression.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        SelectNullExpression.name(),
        "Ambiguous/SelectNullExpression"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM (((");
    assert!(diags.is_empty());
}

// ── SELECT NULL unnamed → 1 violation ────────────────────────────────────────

#[test]
fn select_null_unnamed_one_violation() {
    let diags = check("SELECT NULL FROM t");
    assert_eq!(diags.len(), 1);
}

// ── SELECT NULL AS alias → no violation ──────────────────────────────────────

#[test]
fn select_null_with_alias_no_violation() {
    let diags = check("SELECT NULL AS placeholder FROM t");
    assert!(diags.is_empty());
}

// ── SELECT regular column → no violation ─────────────────────────────────────

#[test]
fn select_column_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

// ── SELECT id, NULL, name → 1 violation (the NULL) ───────────────────────────

#[test]
fn select_null_with_other_columns_violation() {
    let diags = check("SELECT id, NULL, name FROM t");
    assert_eq!(diags.len(), 1);
}

// ── SELECT CASE WHEN x THEN NULL ELSE 1 END → no violation ───────────────────

#[test]
fn select_null_in_case_no_violation() {
    let diags = check("SELECT CASE WHEN x = 1 THEN NULL ELSE 1 END FROM t");
    assert!(diags.is_empty());
}

// ── SELECT NULL, NULL → 2 violations ─────────────────────────────────────────

#[test]
fn two_select_null_two_violations() {
    let diags = check("SELECT NULL, NULL FROM t");
    assert_eq!(diags.len(), 2);
}

// ── NULL in WHERE clause (IS NULL) → no violation ────────────────────────────

#[test]
fn null_in_where_no_violation() {
    let diags = check("SELECT id FROM t WHERE col IS NULL");
    assert!(diags.is_empty());
}

// ── SELECT NULL with no FROM → 1 violation ───────────────────────────────────

#[test]
fn select_null_no_from_violation() {
    let diags = check("SELECT NULL");
    assert_eq!(diags.len(), 1);
}

// ── Message contains useful text ─────────────────────────────────────────────

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT NULL FROM t");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_lowercase().contains("null"),
        "message should mention NULL; got: {}",
        diags[0].message
    );
    assert!(
        diags[0].message.to_lowercase().contains("alias"),
        "message should mention alias; got: {}",
        diags[0].message
    );
}

// ── Line and col are >= 1 ─────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT NULL FROM t");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── Subquery with SELECT NULL → violation ────────────────────────────────────

#[test]
fn subquery_with_select_null_violation() {
    let diags = check("SELECT id FROM (SELECT NULL FROM t) sub");
    assert_eq!(diags.len(), 1);
}
