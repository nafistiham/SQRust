use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::too_many_joins::TooManyJoins;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManyJoins::default().check(&c)
}

fn check_with(sql: &str, max_joins: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManyJoins { max_joins }.check(&c)
}

/// Build a SQL string with `n` JOINs:
///   SELECT * FROM t1 JOIN t2 ON t1.id = t2.id JOIN t3 ON ... ...
fn make_joins(n: usize) -> String {
    let mut sql = "SELECT * FROM t1".to_string();
    for i in 2..=n + 1 {
        sql.push_str(&format!(" JOIN t{i} ON t1.id = t{i}.id"));
    }
    sql
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(TooManyJoins::default().name(), "TooManyJoins");
}

// ── default max_joins ─────────────────────────────────────────────────────────

#[test]
fn default_max_joins_is_five() {
    assert_eq!(TooManyJoins::default().max_joins, 5);
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM JOIN BROKEN WHERE");
    assert!(diags.is_empty());
}

// ── no FROM clause → 0 violations ────────────────────────────────────────────

#[test]
fn no_from_clause_no_violation() {
    let diags = check("SELECT 1");
    assert!(diags.is_empty());
}

// ── SELECT * FROM t (no joins) → 0 violations ────────────────────────────────

#[test]
fn select_from_no_joins_no_violation() {
    let diags = check("SELECT * FROM t");
    assert!(diags.is_empty());
}

// ── 0 joins → 0 violations ───────────────────────────────────────────────────

#[test]
fn zero_joins_no_violation() {
    let diags = check(&make_joins(0));
    assert!(diags.is_empty());
}

// ── 3 joins with default max 5 → 0 violations ────────────────────────────────

#[test]
fn three_joins_default_max_no_violation() {
    let diags = check(&make_joins(3));
    assert!(diags.is_empty());
}

// ── 5 joins with default max 5 → 0 violations (at limit, not over) ───────────

#[test]
fn five_joins_at_default_max_no_violation() {
    let diags = check(&make_joins(5));
    assert!(diags.is_empty());
}

// ── 6 joins with default max 5 → 1 violation ─────────────────────────────────

#[test]
fn six_joins_over_default_max_one_violation() {
    let diags = check(&make_joins(6));
    assert_eq!(diags.len(), 1);
}

// ── custom max_joins: 2 with 3 joins → 1 violation ───────────────────────────

#[test]
fn custom_max_2_three_joins_one_violation() {
    let diags = check_with(&make_joins(3), 2);
    assert_eq!(diags.len(), 1);
}

// ── custom max_joins: 2 with 2 joins → 0 violations ──────────────────────────

#[test]
fn custom_max_2_two_joins_no_violation() {
    let diags = check_with(&make_joins(2), 2);
    assert!(diags.is_empty());
}

// ── message contains join count and max ──────────────────────────────────────

#[test]
fn message_contains_count_and_max() {
    let diags = check(&make_joins(6));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('6'),
        "message should contain the join count"
    );
    assert!(
        diags[0].message.contains('5'),
        "message should contain the max"
    );
}

// ── 10 joins → 1 violation (default max) ─────────────────────────────────────

#[test]
fn ten_joins_default_max_one_violation() {
    let diags = check(&make_joins(10));
    assert_eq!(diags.len(), 1);
}

// ── custom max_joins: 0 with 1 join → 1 violation ────────────────────────────

#[test]
fn custom_max_0_one_join_one_violation() {
    let diags = check_with(&make_joins(1), 0);
    assert_eq!(diags.len(), 1);
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_is_nonzero() {
    let diags = check(&make_joins(6));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
