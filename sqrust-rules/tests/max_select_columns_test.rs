use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::max_select_columns::MaxSelectColumns;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    MaxSelectColumns::default().check(&c)
}

fn check_with(sql: &str, max_columns: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    MaxSelectColumns { max_columns }.check(&c)
}

// ── rule name ────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(MaxSelectColumns::default().name(), "Structure/MaxSelectColumns");
}

// ── default max_columns ───────────────────────────────────────────────────────

#[test]
fn default_max_columns_is_twenty() {
    assert_eq!(MaxSelectColumns::default().max_columns, 20);
}

// ── simple selects well under default ─────────────────────────────────────────

#[test]
fn two_columns_no_violation() {
    let diags = check("SELECT col1, col2 FROM t");
    assert!(diags.is_empty());
}

// ── wildcard is NOT counted ───────────────────────────────────────────────────

#[test]
fn wildcard_not_counted_no_violation() {
    // SELECT * should not count toward the column limit.
    let diags = check_with("SELECT * FROM t", 1);
    assert!(diags.is_empty());
}

#[test]
fn qualified_wildcard_not_counted_no_violation() {
    // SELECT t.* should not count toward the column limit.
    let diags = check_with("SELECT t.* FROM t", 1);
    assert!(diags.is_empty());
}

// ── at the limit ─────────────────────────────────────────────────────────────

#[test]
fn exactly_twenty_columns_no_violation() {
    let cols: Vec<String> = (1..=20).map(|i| format!("col{i}")).collect();
    let sql = format!("SELECT {} FROM t", cols.join(", "));
    let diags = check(&sql);
    assert!(diags.is_empty());
}

// ── one over the limit ────────────────────────────────────────────────────────

#[test]
fn twenty_one_columns_one_violation() {
    let cols: Vec<String> = (1..=21).map(|i| format!("col{i}")).collect();
    let sql = format!("SELECT {} FROM t", cols.join(", "));
    let diags = check(&sql);
    assert_eq!(diags.len(), 1);
}

// ── custom max_columns ────────────────────────────────────────────────────────

#[test]
fn custom_max_three_three_columns_no_violation() {
    let diags = check_with("SELECT col1, col2, col3 FROM t", 3);
    assert!(diags.is_empty());
}

#[test]
fn custom_max_three_four_columns_one_violation() {
    let diags = check_with("SELECT col1, col2, col3, col4 FROM t", 3);
    assert_eq!(diags.len(), 1);
}

// ── message format ────────────────────────────────────────────────────────────

#[test]
fn violation_message_contains_count_and_max() {
    let diags = check_with("SELECT col1, col2, col3, col4 FROM t", 3);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains('4'), "message should contain the column count");
    assert!(diags[0].message.contains('3'), "message should contain the max");
}

// ── diagnostic rule field ──────────────────────────────────────────────────────

#[test]
fn diagnostic_rule_field_is_correct() {
    let diags = check_with("SELECT col1, col2, col3, col4 FROM t", 3);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/MaxSelectColumns");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_empty() {
    let c = ctx("SELECTT INVALID GARBAGE @@##");
    if !c.parse_errors.is_empty() {
        let diags = MaxSelectColumns::default().check(&c);
        assert!(diags.is_empty());
    }
}

// ── multiple SELECTs ──────────────────────────────────────────────────────────

#[test]
fn multiple_selects_only_violators_flagged() {
    let diags = check_with(
        "SELECT col1, col2 FROM t1; SELECT col1, col2, col3, col4 FROM t2",
        3,
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_violating_selects_two_violations() {
    let diags = check_with(
        "SELECT col1, col2, col3, col4 FROM t1; SELECT col1, col2, col3, col4 FROM t2",
        3,
    );
    assert_eq!(diags.len(), 2);
}

// ── subquery ──────────────────────────────────────────────────────────────────

#[test]
fn subquery_with_many_columns_is_flagged() {
    let diags = check_with(
        "SELECT sub.col1 FROM (SELECT col1, col2, col3, col4 FROM t) sub",
        3,
    );
    assert_eq!(diags.len(), 1);
}
