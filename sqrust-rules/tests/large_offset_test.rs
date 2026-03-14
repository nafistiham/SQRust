use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::large_offset::LargeOffset;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    LargeOffset.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(LargeOffset.name(), "Structure/LargeOffset");
}

// ── basic violations ──────────────────────────────────────────────────────────

#[test]
fn offset_1001_one_violation() {
    let diags = check("SELECT * FROM t LIMIT 10 OFFSET 1001");
    assert_eq!(diags.len(), 1);
}

#[test]
fn offset_10000_one_violation() {
    let diags = check("SELECT * FROM t LIMIT 10 OFFSET 10000");
    assert_eq!(diags.len(), 1);
}

// ── boundary: exactly 1000 is fine ────────────────────────────────────────────

#[test]
fn offset_1000_no_violation() {
    let diags = check("SELECT * FROM t LIMIT 10 OFFSET 1000");
    assert!(diags.is_empty());
}

// ── small offsets — no violation ──────────────────────────────────────────────

#[test]
fn offset_100_no_violation() {
    let diags = check("SELECT * FROM t LIMIT 10 OFFSET 100");
    assert!(diags.is_empty());
}

#[test]
fn offset_zero_no_violation() {
    let diags = check("SELECT * FROM t LIMIT 10 OFFSET 0");
    assert!(diags.is_empty());
}

#[test]
fn offset_999_no_violation() {
    let diags = check("SELECT * FROM t LIMIT 10 OFFSET 999");
    assert!(diags.is_empty());
}

// ── no offset — no violation ──────────────────────────────────────────────────

#[test]
fn no_offset_no_violation() {
    let diags = check("SELECT * FROM t LIMIT 10");
    assert!(diags.is_empty());
}

#[test]
fn limit_without_offset_no_violation() {
    let diags = check("SELECT id, name FROM t ORDER BY id LIMIT 100");
    assert!(diags.is_empty());
}

// ── message content ───────────────────────────────────────────────────────────

#[test]
fn offset_1001_message_content() {
    let diags = check("SELECT * FROM t LIMIT 10 OFFSET 1001");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("1001"),
        "expected message to mention the offset value, got: {}",
        diags[0].message
    );
}

// ── CTE and subquery recursion ────────────────────────────────────────────────

#[test]
fn offset_in_cte_violation() {
    let diags = check(
        "WITH c AS (SELECT * FROM t LIMIT 10 OFFSET 2000) SELECT * FROM c",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn offset_in_subquery_violation() {
    let diags = check(
        "SELECT * FROM (SELECT * FROM t LIMIT 5 OFFSET 5000) sub",
    );
    assert_eq!(diags.len(), 1);
}

// ── multiple queries ──────────────────────────────────────────────────────────

#[test]
fn two_queries_two_violations() {
    let diags = check(
        "SELECT * FROM t LIMIT 10 OFFSET 2000; SELECT * FROM u LIMIT 10 OFFSET 3000",
    );
    assert_eq!(diags.len(), 2);
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_no_violations() {
    let diags = check("SELECT FROM FROM WHERE");
    assert!(diags.is_empty());
}

// ── line/col are nonzero ──────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT * FROM t LIMIT 10 OFFSET 1001");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
