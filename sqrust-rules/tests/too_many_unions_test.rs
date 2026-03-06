use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::too_many_unions::TooManyUnions;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManyUnions::default().check(&c)
}

fn check_with(sql: &str, max_unions: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManyUnions { max_unions }.check(&c)
}

/// Build a UNION ALL chain with `n` SELECT arms.
/// make_union(1)  → SELECT 1                                     (0 set ops)
/// make_union(2)  → SELECT 1 UNION ALL SELECT 2                  (1 set op)
/// make_union(4)  → SELECT 1 UNION ALL ... UNION ALL SELECT 4    (3 set ops)
fn make_union(n: usize) -> String {
    let arms: Vec<String> = (1..=n).map(|i| format!("SELECT {i}")).collect();
    arms.join(" UNION ALL ")
}

/// Build a chain using INTERSECT.
fn make_intersect(n: usize) -> String {
    let arms: Vec<String> = (1..=n).map(|i| format!("SELECT {i}")).collect();
    arms.join(" INTERSECT ")
}

/// Build a chain using EXCEPT.
fn make_except(n: usize) -> String {
    let arms: Vec<String> = (1..=n).map(|i| format!("SELECT {i}")).collect();
    arms.join(" EXCEPT ")
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(TooManyUnions::default().name(), "TooManyUnions");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM UNION BROKEN WHERE");
    assert!(diags.is_empty());
}

// ── default max ───────────────────────────────────────────────────────────────

#[test]
fn default_max_is_three() {
    assert_eq!(TooManyUnions::default().max_unions, 3);
}

// ── at max: 4 SELECTs = 3 set ops → no violation ─────────────────────────────

#[test]
fn three_unions_at_max_no_violation() {
    let diags = check(&make_union(4)); // 3 UNION ALLs
    assert!(diags.is_empty());
}

// ── over max: 5 SELECTs = 4 set ops → 1 violation ───────────────────────────

#[test]
fn four_unions_over_max_one_violation() {
    let diags = check(&make_union(5)); // 4 UNION ALLs > 3
    assert_eq!(diags.len(), 1);
}

// ── under max: 2 set ops → no violation ──────────────────────────────────────

#[test]
fn two_unions_under_max_no_violation() {
    let diags = check(&make_union(3)); // 2 UNION ALLs
    assert!(diags.is_empty());
}

// ── custom max 2, 3 set ops → 1 violation ────────────────────────────────────

#[test]
fn custom_max_2_with_3_unions_one_violation() {
    let diags = check_with(&make_union(4), 2); // 3 set ops > max 2
    assert_eq!(diags.len(), 1);
}

// ── custom max 2, 2 set ops → no violation ───────────────────────────────────

#[test]
fn custom_max_2_with_2_unions_no_violation() {
    let diags = check_with(&make_union(3), 2); // 2 set ops == max 2
    assert!(diags.is_empty());
}

// ── single SELECT → no violation ─────────────────────────────────────────────

#[test]
fn single_select_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

// ── message contains count and max ───────────────────────────────────────────

#[test]
fn message_contains_count_and_max() {
    // make_union(5) → 4 set ops, max 3
    let diags = check(&make_union(5));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('4'),
        "message should contain the set op count (4)"
    );
    assert!(
        diags[0].message.contains('3'),
        "message should contain the max (3)"
    );
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check(&make_union(5));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── INTERSECT counts toward limit ────────────────────────────────────────────

#[test]
fn intersect_counts_toward_limit() {
    // 5 arms, 4 INTERSECTs > max 3 → flag
    let diags = check(&make_intersect(5));
    assert_eq!(diags.len(), 1);
}

// ── EXCEPT counts toward limit ───────────────────────────────────────────────

#[test]
fn except_counts_toward_limit() {
    // 5 arms, 4 EXCEPTs > max 3 → flag
    let diags = check(&make_except(5));
    assert_eq!(diags.len(), 1);
}

// ── mixed UNION and UNION ALL combined count ─────────────────────────────────

#[test]
fn union_and_union_all_combined_count() {
    // 5 arms with mixed UNION / UNION ALL → 4 set ops > 3 → flag
    let sql = "SELECT 1 UNION SELECT 2 UNION ALL SELECT 3 UNION SELECT 4 UNION ALL SELECT 5";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

// ── INTERSECT at max → no violation ──────────────────────────────────────────

#[test]
fn intersect_at_max_no_violation() {
    // 4 arms, 3 INTERSECTs == max 3 → ok
    let diags = check(&make_intersect(4));
    assert!(diags.is_empty());
}
