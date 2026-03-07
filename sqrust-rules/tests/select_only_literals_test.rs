use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::select_only_literals::SelectOnlyLiterals;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    SelectOnlyLiterals::default().check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        SelectOnlyLiterals::default().name(),
        "Structure/SelectOnlyLiterals"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM FROM BROKEN");
    assert!(diags.is_empty());
}

// ── SELECT integer with no FROM — 1 violation ─────────────────────────────────

#[test]
fn select_integer_no_from_one_violation() {
    let diags = check("SELECT 1");
    assert_eq!(
        diags.len(),
        1,
        "SELECT of a single integer literal should be flagged"
    );
}

// ── SELECT string with no FROM — 1 violation ──────────────────────────────────

#[test]
fn select_string_no_from_one_violation() {
    let diags = check("SELECT 'hello'");
    assert_eq!(
        diags.len(),
        1,
        "SELECT of a single string literal should be flagged"
    );
}

// ── SELECT multiple literals with no FROM — 1 violation ───────────────────────

#[test]
fn select_multiple_literals_no_from_one_violation() {
    let diags = check("SELECT 1, 'a', TRUE");
    assert_eq!(
        diags.len(),
        1,
        "SELECT of multiple literal values should produce 1 violation"
    );
}

// ── SELECT with FROM — no violation ───────────────────────────────────────────

#[test]
fn select_with_from_no_violation() {
    // Even though projection is a literal, FROM clause is present — should not flag
    let diags = check("SELECT 1 FROM t");
    assert!(
        diags.is_empty(),
        "SELECT with FROM clause should not be flagged"
    );
}

// ── SELECT column (no FROM) — no violation ────────────────────────────────────

#[test]
fn select_column_no_from_no_violation() {
    // Column reference — not a literal — should not flag even without FROM
    let diags = check("SELECT id");
    assert!(
        diags.is_empty(),
        "SELECT of a column reference should not be flagged"
    );
}

// ── SELECT function call with no FROM — no violation ──────────────────────────

#[test]
fn select_function_no_from_no_violation() {
    let diags = check("SELECT NOW()");
    assert!(
        diags.is_empty(),
        "SELECT of a function call should not be flagged"
    );
}

// ── SELECT column FROM table — no violation ───────────────────────────────────

#[test]
fn select_column_from_table_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

// ── SELECT NULL with no FROM — 1 violation ────────────────────────────────────

#[test]
fn select_null_no_from_one_violation() {
    let diags = check("SELECT NULL");
    assert_eq!(
        diags.len(),
        1,
        "SELECT NULL with no FROM should be flagged"
    );
}

// ── two literal-only SELECT statements — 2 violations ─────────────────────────

#[test]
fn two_literal_selects_two_violations() {
    let diags = check("SELECT 1; SELECT 'x'");
    assert_eq!(
        diags.len(),
        2,
        "Two literal-only SELECTs should produce 2 violations"
    );
}

// ── message mentions literal or test ─────────────────────────────────────────

#[test]
fn message_mentions_literal_or_test() {
    let diags = check("SELECT 1");
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("literal") || msg.contains("test"),
        "message should mention 'literal' or 'test': got '{}'",
        diags[0].message
    );
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT 1");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── SELECT expression (1 + 2) with no FROM — no violation ─────────────────────

#[test]
fn select_expression_no_from_no_violation() {
    // BinaryOp, not a pure Expr::Value — should not flag
    let diags = check("SELECT 1 + 2");
    assert!(
        diags.is_empty(),
        "SELECT of a binary expression should not be flagged"
    );
}
