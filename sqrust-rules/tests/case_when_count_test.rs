use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::case_when_count::CaseWhenCount;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    CaseWhenCount::default().check(&c)
}

fn check_with(sql: &str, max_when_clauses: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    CaseWhenCount { max_when_clauses }.check(&c)
}

/// Build a SQL string with `n` WHEN branches:
///   SELECT CASE WHEN 1=1 THEN 'a' WHEN 2=2 THEN 'b' ... END
fn make_case(n: usize) -> String {
    if n == 0 {
        return "SELECT 1".to_string();
    }
    let branches: Vec<String> = (1..=n)
        .map(|i| format!("WHEN {i}=1 THEN '{i}'"))
        .collect();
    format!("SELECT CASE {} ELSE 'x' END", branches.join(" "))
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(CaseWhenCount::default().name(), "CaseWhenCount");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT CASE FROM WHEN BROKEN");
    assert!(diags.is_empty());
}

// ── CASE with 3 WHENs (default max 5) → 0 violations ─────────────────────────

#[test]
fn three_whens_default_max_no_violation() {
    let diags = check(&make_case(3));
    assert!(diags.is_empty());
}

// ── CASE with 5 WHENs (default max 5) → 0 violations (at limit) ──────────────

#[test]
fn five_whens_at_default_max_no_violation() {
    let diags = check(&make_case(5));
    assert!(diags.is_empty());
}

// ── CASE with 6 WHENs (default max 5) → 1 violation ─────────────────────────

#[test]
fn six_whens_over_default_max_one_violation() {
    let diags = check(&make_case(6));
    assert_eq!(diags.len(), 1);
}

// ── custom max_when_clauses: 2 with 3 WHENs → 1 violation ────────────────────

#[test]
fn custom_max_2_three_whens_one_violation() {
    let diags = check_with(&make_case(3), 2);
    assert_eq!(diags.len(), 1);
}

// ── custom max_when_clauses: 2 with 2 WHENs → 0 violations ───────────────────

#[test]
fn custom_max_2_two_whens_no_violation() {
    let diags = check_with(&make_case(2), 2);
    assert!(diags.is_empty());
}

// ── custom max_when_clauses: 0 with 1 WHEN → 1 violation ─────────────────────

#[test]
fn custom_max_0_one_when_one_violation() {
    let diags = check_with(&make_case(1), 0);
    assert_eq!(diags.len(), 1);
}

// ── default max_when_clauses = 5 ─────────────────────────────────────────────

#[test]
fn default_max_when_clauses_is_five() {
    assert_eq!(CaseWhenCount::default().max_when_clauses, 5);
}

// ── message contains actual count and max ────────────────────────────────────

#[test]
fn message_contains_count_and_max() {
    let diags = check(&make_case(6));
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('6'),
        "message should contain the WHEN count"
    );
    assert!(
        diags[0].message.contains('5'),
        "message should contain the max"
    );
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_is_nonzero() {
    let diags = check(&make_case(6));
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── no CASE expression → 0 violations ────────────────────────────────────────

#[test]
fn no_case_expression_no_violation() {
    let diags = check("SELECT id, name FROM t WHERE id > 1");
    assert!(diags.is_empty());
}

// ── nested CASE: outer OK, inner exceeds → 1 violation ───────────────────────

#[test]
fn nested_case_inner_exceeds_one_violation() {
    // Outer CASE has 2 WHENs (ok with default max 5).
    // Inner CASE has 6 WHENs (exceeds max 5).
    let inner = make_case(6);
    // Extract just the CASE...END portion from make_case
    let inner_case = inner.trim_start_matches("SELECT ").to_string();
    let sql = format!(
        "SELECT CASE WHEN 1=1 THEN ({inner_case}) WHEN 2=2 THEN 'b' ELSE 'x' END"
    );
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
}

// ── both nested CASEs exceed → 2 violations ──────────────────────────────────

#[test]
fn two_case_expressions_both_exceed_two_violations() {
    // Two independent CASE expressions in the same SELECT, each with 6 WHENs.
    let case6 = make_case(6)
        .trim_start_matches("SELECT ")
        .to_string();
    let sql = format!("SELECT {case6}, {case6}");
    let diags = check(&sql);
    assert_eq!(diags.len(), 2);
}
