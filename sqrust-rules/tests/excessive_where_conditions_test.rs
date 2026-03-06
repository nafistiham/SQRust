use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::excessive_where_conditions::ExcessiveWhereConditions;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    ExcessiveWhereConditions::default().check(&c)
}

fn check_with(sql: &str, max_conditions: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    ExcessiveWhereConditions { max_conditions }.check(&c)
}

/// Build a SELECT with `n` conditions connected by AND operators.
/// make_where(1)  → WHERE a1=1               (0 operators)
/// make_where(2)  → WHERE a1=1 AND a2=2      (1 operator)
/// make_where(11) → WHERE a1=1 AND ... AND a11=11  (10 operators)
fn make_where(n: usize) -> String {
    let conditions: Vec<String> = (1..=n).map(|i| format!("a{i}={i}")).collect();
    format!("SELECT * FROM t WHERE {}", conditions.join(" AND "))
}

/// Build a SELECT with `n` conditions connected by OR operators.
fn make_where_or(n: usize) -> String {
    let conditions: Vec<String> = (1..=n).map(|i| format!("a{i}={i}")).collect();
    format!("SELECT * FROM t WHERE {}", conditions.join(" OR "))
}

/// Build a query with HAVING clause with `n` conditions connected by AND.
fn make_having(n: usize) -> String {
    let conditions: Vec<String> = (1..=n).map(|i| format!("COUNT(*)>{i}")).collect();
    format!(
        "SELECT id FROM t GROUP BY id HAVING {}",
        conditions.join(" AND ")
    )
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        ExcessiveWhereConditions::default().name(),
        "ExcessiveWhereConditions"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM WHERE BROKEN AND");
    assert!(diags.is_empty());
}

// ── default max ───────────────────────────────────────────────────────────────

#[test]
fn default_max_is_ten() {
    assert_eq!(ExcessiveWhereConditions::default().max_conditions, 10);
}

// ── at max: 11 conditions = 10 AND operators → no violation ──────────────────
// make_where(11) → 11 conditions, 10 AND operators. 10 == max → no flag.

#[test]
fn ten_operators_at_max_no_violation() {
    let diags = check(&make_where(11));
    assert!(diags.is_empty(), "10 AND operators should not trigger at max=10");
}

// ── over max: 12 conditions = 11 AND operators → 1 violation ─────────────────
// make_where(12) → 12 conditions, 11 AND operators. 11 > 10 → flag.

#[test]
fn eleven_operators_over_max_one_violation() {
    let diags = check(&make_where(12));
    assert_eq!(diags.len(), 1);
}

// ── under max: 5 AND operators → no violation ────────────────────────────────

#[test]
fn under_max_no_violation() {
    let diags = check(&make_where(6)); // 5 operators
    assert!(diags.is_empty());
}

// ── custom max 3, 4 operators → 1 violation ──────────────────────────────────

#[test]
fn custom_max_3_with_4_operators_one_violation() {
    // make_where(5) → 4 operators > max 3 → flag
    let diags = check_with(&make_where(5), 3);
    assert_eq!(diags.len(), 1);
}

// ── custom max 3, 3 operators → no violation ─────────────────────────────────

#[test]
fn custom_max_3_with_3_operators_no_violation() {
    // make_where(4) → 3 operators == max 3 → ok
    let diags = check_with(&make_where(4), 3);
    assert!(diags.is_empty());
}

// ── no WHERE clause → no violation ───────────────────────────────────────────

#[test]
fn no_where_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

// ── OR conditions also counted ────────────────────────────────────────────────

#[test]
fn or_conditions_counted() {
    // make_where_or(12) → 11 OR operators > 10 → flag
    let diags = check(&make_where_or(12));
    assert_eq!(diags.len(), 1);
}

// ── message contains count and max ───────────────────────────────────────────

#[test]
fn message_contains_count_and_max() {
    // make_where(12) → 11 operators, max 10
    let diags = check(&make_where(12));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("11"),
        "message should contain the operator count (11)"
    );
    assert!(
        diags[0].message.contains("10"),
        "message should contain the max (10)"
    );
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check(&make_where(12));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── HAVING clause with excessive conditions → violation ───────────────────────

#[test]
fn having_with_too_many_conditions_violation() {
    // make_having(12) → 11 AND operators > 10 → flag
    let diags = check(&make_having(12));
    assert_eq!(diags.len(), 1);
}

// ── single condition → no violation ─────────────────────────────────────────

#[test]
fn single_condition_no_violation() {
    let diags = check("SELECT * FROM t WHERE a=1");
    assert!(diags.is_empty());
}

// ── having at max → no violation ─────────────────────────────────────────────

#[test]
fn having_at_max_no_violation() {
    // make_having(11) → 10 AND operators == max 10 → ok
    let diags = check(&make_having(11));
    assert!(diags.is_empty());
}
