use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::max_join_on_conditions::MaxJoinOnConditions;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    MaxJoinOnConditions::default().check(&c)
}

fn check_with(sql: &str, max_conditions: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    MaxJoinOnConditions { max_conditions }.check(&c)
}

/// Build a JOIN ON clause with `n` conditions connected by AND.
fn make_join(n: usize) -> String {
    let conditions: Vec<String> = (1..=n).map(|i| format!("a.col{i} = b.col{i}")).collect();
    format!(
        "SELECT a.id FROM a INNER JOIN b ON {}",
        conditions.join(" AND ")
    )
}

/// Build two JOINs: first with `n1` conditions, second with `n2` conditions.
fn make_two_joins(n1: usize, n2: usize) -> String {
    let conds1: Vec<String> = (1..=n1).map(|i| format!("a.col{i} = b.col{i}")).collect();
    let conds2: Vec<String> = (1..=n2).map(|i| format!("a.col{i} = c.col{i}")).collect();
    format!(
        "SELECT a.id FROM a INNER JOIN b ON {} LEFT JOIN c ON {}",
        conds1.join(" AND "),
        conds2.join(" AND ")
    )
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        MaxJoinOnConditions::default().name(),
        "Structure/MaxJoinOnConditions"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM JOIN BROKEN ON");
    assert!(diags.is_empty());
}

// ── one condition at default max — no violation ───────────────────────────────

#[test]
fn one_condition_default_max_no_violation() {
    let diags = check(&make_join(1));
    assert!(
        diags.is_empty(),
        "1 condition should not trigger at max=3"
    );
}

// ── exactly 3 conditions at default max — no violation ───────────────────────

#[test]
fn three_conditions_at_default_max_no_violation() {
    let diags = check(&make_join(3));
    assert!(
        diags.is_empty(),
        "3 conditions at max=3 should not trigger"
    );
}

// ── 4 conditions over default max — 1 violation ───────────────────────────────

#[test]
fn four_conditions_over_default_max_one_violation() {
    let diags = check(&make_join(4));
    assert_eq!(
        diags.len(),
        1,
        "4 conditions over max=3 should produce 1 violation"
    );
}

// ── two joins: one complex, one simple — 1 violation ─────────────────────────

#[test]
fn two_joins_one_complex_one_simple_one_violation() {
    // first join: 4 conditions (flagged), second join: 2 conditions (not flagged)
    let diags = check(&make_two_joins(4, 2));
    assert_eq!(
        diags.len(),
        1,
        "Only the complex JOIN should be flagged; got {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// ── two joins: both complex — 2 violations ───────────────────────────────────

#[test]
fn two_joins_both_complex_two_violations() {
    // both joins have 4 conditions — both flagged
    let diags = check(&make_two_joins(4, 4));
    assert_eq!(
        diags.len(),
        2,
        "Both complex JOINs should be flagged"
    );
}

// ── no JOIN — no violation ────────────────────────────────────────────────────

#[test]
fn no_join_no_violation() {
    let diags = check("SELECT id FROM t WHERE id = 1");
    assert!(diags.is_empty());
}

// ── custom max=2: 3 conditions — 1 violation ─────────────────────────────────

#[test]
fn custom_max_2_three_conditions_one_violation() {
    let diags = check_with(&make_join(3), 2);
    assert_eq!(diags.len(), 1);
}

// ── custom max=2: 2 conditions — no violation ────────────────────────────────

#[test]
fn custom_max_2_two_conditions_no_violation() {
    let diags = check_with(&make_join(2), 2);
    assert!(diags.is_empty());
}

// ── default max is 3 ─────────────────────────────────────────────────────────

#[test]
fn default_max_is_three() {
    assert_eq!(MaxJoinOnConditions::default().max_conditions, 3);
}

// ── message contains count and max ───────────────────────────────────────────

#[test]
fn message_contains_count_and_max() {
    // 4 conditions, max 3
    let diags = check(&make_join(4));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('4'),
        "message should contain the condition count (4): got '{}'",
        diags[0].message
    );
    assert!(
        diags[0].message.contains('3'),
        "message should contain the max (3): got '{}'",
        diags[0].message
    );
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check(&make_join(4));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── INNER JOIN and LEFT JOIN each independently flagged ───────────────────────

#[test]
fn inner_join_and_left_join_each_flagged() {
    // first join INNER (4 conds), second LEFT (4 conds) — both should be flagged
    let conds: Vec<String> = (1..=4).map(|i| format!("a.col{i} = b.col{i}")).collect();
    let conds2: Vec<String> = (1..=4).map(|i| format!("a.col{i} = c.col{i}")).collect();
    let sql = format!(
        "SELECT a.id FROM a INNER JOIN b ON {} LEFT JOIN c ON {}",
        conds.join(" AND "),
        conds2.join(" AND ")
    );
    let diags = check(&sql);
    assert_eq!(diags.len(), 2, "Both INNER and LEFT JOINs should be flagged");
}
