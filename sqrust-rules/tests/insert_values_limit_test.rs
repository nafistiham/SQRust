use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::insert_values_limit::InsertValuesLimit;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    InsertValuesLimit.check(&c)
}

fn make_insert_sql(n: usize) -> String {
    let rows: Vec<String> = (1..=n).map(|i| format!("({})", i)).collect();
    format!("INSERT INTO t (a) VALUES {}", rows.join(", "))
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(InsertValuesLimit.name(), "Structure/InsertValuesLimit");
}

// ── 50 rows — no violation ────────────────────────────────────────────────────

#[test]
fn insert_50_rows_no_violation() {
    let sql = make_insert_sql(50);
    let diags = check(&sql);
    assert!(diags.is_empty(), "50 rows should not trigger a violation");
}

// ── 51 rows — 1 violation ─────────────────────────────────────────────────────

#[test]
fn insert_51_rows_one_violation() {
    let sql = make_insert_sql(51);
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
}

// ── 100 rows — 1 violation ────────────────────────────────────────────────────

#[test]
fn insert_100_rows_one_violation() {
    let sql = make_insert_sql(100);
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
}

// ── 1 row — no violation ─────────────────────────────────────────────────────

#[test]
fn insert_1_row_no_violation() {
    let diags = check("INSERT INTO t (a) VALUES (1)");
    assert!(diags.is_empty());
}

// ── 0 rows edge case (invalid SQL but handle gracefully) ──────────────────────

#[test]
fn insert_0_rows_no_violation() {
    // INSERT INTO t (a) VALUES with no rows is invalid SQL — parser will fail.
    // The rule should return 0 violations gracefully.
    let diags = check("INSERT INTO t (a) VALUES");
    assert!(diags.is_empty());
}

// ── INSERT ... SELECT (not VALUES form) — no violation ───────────────────────

#[test]
fn insert_with_select_no_violation() {
    let diags = check("INSERT INTO t SELECT * FROM s");
    assert!(diags.is_empty());
}

// ── two inserts: one with 60 rows (violation), one with 10 rows (ok) ─────────

#[test]
fn two_inserts_one_over_limit() {
    let big = make_insert_sql(60);
    let small = make_insert_sql(10);
    let sql = format!("{}; {}", big, small);
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
}

// ── message mentions row count ─────────────────────────────────────────────────

#[test]
fn message_mentions_row_count() {
    let sql = make_insert_sql(51);
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("51"),
        "message should mention the row count: {}",
        diags[0].message
    );
}

// ── message mentions limit ────────────────────────────────────────────────────

#[test]
fn message_mentions_limit() {
    let sql = make_insert_sql(51);
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("50"),
        "message should mention the limit of 50: {}",
        diags[0].message
    );
}

// ── parse error returns no violations ────────────────────────────────────────

#[test]
fn parse_error_no_violations() {
    let diags = check("INSERT INTO BROKEN VALUES SELECT FROM @@");
    assert!(diags.is_empty());
}

// ── line and col are non-zero ─────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let sql = make_insert_sql(51);
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line should be >= 1");
    assert!(diags[0].col >= 1, "col should be >= 1");
}

// ── 49 rows — no violation ────────────────────────────────────────────────────

#[test]
fn insert_49_rows_no_violation() {
    let sql = make_insert_sql(49);
    let diags = check(&sql);
    assert!(diags.is_empty(), "49 rows should not trigger a violation");
}

// ── exactly 51 rows — exactly 1 violation ────────────────────────────────────

#[test]
fn insert_51_rows_violation_count() {
    let sql = make_insert_sql(51);
    let diags = check(&sql);
    assert_eq!(
        diags.len(),
        1,
        "exactly 51 rows should produce exactly 1 violation"
    );
}
