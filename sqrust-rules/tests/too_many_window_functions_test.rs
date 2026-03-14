use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::too_many_window_functions::TooManyWindowFunctions;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManyWindowFunctions::default().check(&c)
}

fn check_with(sql: &str, max: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManyWindowFunctions { max }.check(&c)
}

/// Build a SELECT with `n` window function calls.
fn make_window_fns(n: usize) -> String {
    let cols: Vec<String> = (1..=n)
        .map(|i| format!("ROW_NUMBER() OVER (ORDER BY id) AS rn{i}"))
        .collect();
    if cols.is_empty() {
        "SELECT id FROM t".to_string()
    } else {
        format!("SELECT {}, id FROM t", cols.join(", "))
    }
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        TooManyWindowFunctions::default().name(),
        "Structure/TooManyWindowFunctions"
    );
}

// ── default max ───────────────────────────────────────────────────────────────

#[test]
fn default_max_is_five() {
    assert_eq!(TooManyWindowFunctions::default().max, 5);
}

// ── parse error → 0 violations ───────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT OVER BROKEN FROM JOIN");
    assert!(diags.is_empty());
}

// ── 0 window fns → 0 violations ──────────────────────────────────────────────

#[test]
fn zero_window_fns_no_violation() {
    let diags = check("SELECT id, name FROM t");
    assert!(diags.is_empty());
}

// ── 3 window fns with default max 5 → 0 violations ───────────────────────────

#[test]
fn three_window_fns_default_max_no_violation() {
    let diags = check(&make_window_fns(3));
    assert!(diags.is_empty());
}

// ── 5 window fns with default max 5 → 0 violations (at limit, not over) ──────

#[test]
fn five_window_fns_at_default_max_no_violation() {
    let diags = check(&make_window_fns(5));
    assert!(diags.is_empty());
}

// ── 6 window fns with default max 5 → 1 violation ────────────────────────────

#[test]
fn six_window_fns_over_default_max_one_violation() {
    let diags = check(&make_window_fns(6));
    assert_eq!(diags.len(), 1);
}

// ── custom max=2, 3 window fns → 1 violation ─────────────────────────────────

#[test]
fn custom_max_2_three_window_fns_one_violation() {
    let diags = check_with(&make_window_fns(3), 2);
    assert_eq!(diags.len(), 1);
}

// ── custom max=2, 2 window fns → 0 violations ────────────────────────────────

#[test]
fn custom_max_2_two_window_fns_no_violation() {
    let diags = check_with(&make_window_fns(2), 2);
    assert!(diags.is_empty());
}

// ── message contains both counts ─────────────────────────────────────────────

#[test]
fn message_contains_window_fn_count_and_max() {
    let diags = check(&make_window_fns(6));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('6'),
        "message should contain the window function count"
    );
    assert!(
        diags[0].message.contains('5'),
        "message should contain the max"
    );
}

// ── diagnostic rule name matches ─────────────────────────────────────────────

#[test]
fn diagnostic_rule_name_matches() {
    let diags = check(&make_window_fns(6));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/TooManyWindowFunctions");
}

// ── window fns inside a CTE are checked ──────────────────────────────────────

#[test]
fn window_fns_in_cte_are_checked() {
    // CTE body has 6 window functions — should be flagged.
    let cols: Vec<String> = (1..=6)
        .map(|i| format!("ROW_NUMBER() OVER (ORDER BY id) AS rn{i}"))
        .collect();
    let sql = format!(
        "WITH cte AS (SELECT {}, id FROM t) SELECT * FROM cte",
        cols.join(", ")
    );
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
}

// ── window fns in subquery are checked ───────────────────────────────────────

#[test]
fn window_fns_in_subquery_are_checked() {
    let cols: Vec<String> = (1..=6)
        .map(|i| format!("ROW_NUMBER() OVER (ORDER BY id) AS rn{i}"))
        .collect();
    let sql = format!(
        "SELECT * FROM (SELECT {}, id FROM t) sub",
        cols.join(", ")
    );
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
}

// ── plain SELECT with aggregate, no window → 0 violations ────────────────────

#[test]
fn aggregate_without_window_no_violation() {
    let diags = check("SELECT COUNT(*), SUM(amount) FROM t GROUP BY category");
    assert!(diags.is_empty());
}
