use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::having_conditions_count::HavingConditionsCount;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    HavingConditionsCount::default().check(&c)
}

fn check_with(sql: &str, max_conditions: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    HavingConditionsCount { max_conditions }.check(&c)
}

/// Build a HAVING clause with `n` conditions connected by AND.
/// make_having(1) → HAVING count(*) > 1              (0 AND ops, 1 condition)
/// make_having(3) → HAVING count(*) > 1 AND ... AND count(*) > 3  (2 AND ops, 3 conditions)
fn make_having(n: usize) -> String {
    let conditions: Vec<String> = (1..=n).map(|i| format!("count(*) > {i}")).collect();
    format!(
        "SELECT id FROM t GROUP BY id HAVING {}",
        conditions.join(" AND ")
    )
}

/// Build a HAVING clause with `n` conditions connected by OR.
fn make_having_or(n: usize) -> String {
    let conditions: Vec<String> = (1..=n).map(|i| format!("count(*) > {i}")).collect();
    format!(
        "SELECT id FROM t GROUP BY id HAVING {}",
        conditions.join(" OR ")
    )
}

/// Build a HAVING clause mixing AND and OR:
/// `a AND b OR c AND d` — 3 binary ops, 4 conditions
fn make_having_mixed(n: usize) -> String {
    let conditions: Vec<String> = (1..=n).map(|i| format!("count(*) > {i}")).collect();
    // Alternate AND/OR between conditions
    let mut parts = conditions[0].clone();
    for (i, cond) in conditions[1..].iter().enumerate() {
        let op = if i % 2 == 0 { "AND" } else { "OR" };
        parts = format!("{parts} {op} {cond}");
    }
    format!("SELECT id FROM t GROUP BY id HAVING {parts}")
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        HavingConditionsCount::default().name(),
        "Structure/HavingConditionsCount"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM HAVING BROKEN AND");
    assert!(diags.is_empty());
}

// ── default max: 3 conditions (2 AND ops) — no violation ─────────────────────

#[test]
fn three_conditions_default_max_no_violation() {
    // 3 conditions = 2 AND ops; max=5 — no flag
    let diags = check(&make_having(3));
    assert!(diags.is_empty(), "3 conditions should not trigger at max=5");
}

// ── default max: 5 conditions (4 AND ops) — exactly at max — no violation ────

#[test]
fn five_conditions_at_default_max_no_violation() {
    // 5 conditions = 4 AND ops; 5 == max=5 — no flag
    let diags = check(&make_having(5));
    assert!(
        diags.is_empty(),
        "5 conditions at max=5 should not trigger"
    );
}

// ── default max: 6 conditions (5 AND ops) — over max — 1 violation ───────────

#[test]
fn six_conditions_over_default_max_one_violation() {
    // 6 conditions = 5 AND ops; 6 > max=5 — flag
    let diags = check(&make_having(6));
    assert_eq!(
        diags.len(),
        1,
        "6 conditions over max=5 should produce 1 violation"
    );
}

// ── no HAVING clause — no violation ──────────────────────────────────────────

#[test]
fn no_having_no_violation() {
    let diags = check("SELECT id, count(*) FROM t GROUP BY id");
    assert!(diags.is_empty());
}

// ── custom max=2: 3 conditions — 1 violation ─────────────────────────────────

#[test]
fn custom_max_2_three_conditions_one_violation() {
    // 3 conditions = 2 AND ops; 3 > max=2 — flag
    let diags = check_with(&make_having(3), 2);
    assert_eq!(diags.len(), 1);
}

// ── custom max=2: 2 conditions — no violation ────────────────────────────────

#[test]
fn custom_max_2_two_conditions_no_violation() {
    // 2 conditions = 1 AND op; 2 == max=2 — no flag
    let diags = check_with(&make_having(2), 2);
    assert!(diags.is_empty());
}

// ── default max is 5 ──────────────────────────────────────────────────────────

#[test]
fn default_max_is_five() {
    assert_eq!(HavingConditionsCount::default().max_conditions, 5);
}

// ── message contains count and max ───────────────────────────────────────────

#[test]
fn message_contains_count_and_max() {
    // 6 conditions, max 5
    let diags = check(&make_having(6));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('6'),
        "message should contain the condition count (6): got '{}'",
        diags[0].message
    );
    assert!(
        diags[0].message.contains('5'),
        "message should contain the max (5): got '{}'",
        diags[0].message
    );
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check(&make_having(6));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── OR conditions also counted ───────────────────────────────────────────────

#[test]
fn or_conditions_counted() {
    // 6 OR-joined conditions = 5 OR ops; 6 > max=5 — flag
    let diags = check(&make_having_or(6));
    assert_eq!(diags.len(), 1, "OR conditions should be counted");
}

// ── mixed AND/OR counted ─────────────────────────────────────────────────────

#[test]
fn mixed_and_or_counted() {
    // make_having_mixed(6) → 6 conditions, 5 binary ops (mix of AND/OR)
    // 6 > max=5 — flag
    let diags = check(&make_having_mixed(6));
    assert_eq!(
        diags.len(),
        1,
        "Mixed AND/OR conditions should be counted correctly"
    );
}

// ── single HAVING condition — no violation ───────────────────────────────────

#[test]
fn single_having_condition_no_violation() {
    // HAVING count(*) > 0 — 1 condition, 0 AND/OR ops; 1 <= max=5 — no flag
    let diags = check("SELECT id FROM t GROUP BY id HAVING count(*) > 0");
    assert!(diags.is_empty(), "A single HAVING condition should not flag");
}
