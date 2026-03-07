use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::too_many_order_by_columns::TooManyOrderByColumns;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManyOrderByColumns::default().check(&c)
}

fn check_with(sql: &str, max_columns: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    TooManyOrderByColumns { max_columns }.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        TooManyOrderByColumns::default().name(),
        "Structure/TooManyOrderByColumns"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM ORDER BROKEN BY");
    assert!(diags.is_empty());
}

// ── 1 column → 0 violations ──────────────────────────────────────────────────

#[test]
fn one_order_by_column_no_violation() {
    let diags = check("SELECT id FROM t ORDER BY id");
    assert!(diags.is_empty());
}

// ── 5 columns at default max → 0 violations ──────────────────────────────────

#[test]
fn five_columns_at_default_max_no_violation() {
    let diags = check("SELECT * FROM t ORDER BY a, b, c, d, e");
    assert!(diags.is_empty());
}

// ── 6 columns over default max → 1 violation ─────────────────────────────────

#[test]
fn six_columns_over_default_one_violation() {
    let diags = check("SELECT * FROM t ORDER BY a, b, c, d, e, f");
    assert_eq!(diags.len(), 1);
}

// ── custom max=2, 3 columns → 1 violation ────────────────────────────────────

#[test]
fn custom_max_2_three_columns_one_violation() {
    let diags = check_with("SELECT * FROM t ORDER BY a, b, c", 2);
    assert_eq!(diags.len(), 1);
}

// ── custom max=2, 2 columns → 0 violations ───────────────────────────────────

#[test]
fn custom_max_2_two_columns_no_violation() {
    let diags = check_with("SELECT * FROM t ORDER BY a, b", 2);
    assert!(diags.is_empty());
}

// ── no ORDER BY → 0 violations ───────────────────────────────────────────────

#[test]
fn no_order_by_no_violation() {
    let diags = check("SELECT * FROM t WHERE id = 1");
    assert!(diags.is_empty());
}

// ── default max is 5 ─────────────────────────────────────────────────────────

#[test]
fn default_max_is_five() {
    assert_eq!(TooManyOrderByColumns::default().max_columns, 5);
}

// ── message contains count and max ───────────────────────────────────────────

#[test]
fn message_contains_count_and_max() {
    let diags = check("SELECT * FROM t ORDER BY a, b, c, d, e, f");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('6'),
        "message should contain the column count (6)"
    );
    assert!(
        diags[0].message.contains('5'),
        "message should contain the max (5)"
    );
}

// ── line is non-zero ──────────────────────────────────────────────────────────

#[test]
fn line_nonzero() {
    let diags = check("SELECT * FROM t ORDER BY a, b, c, d, e, f");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

// ── col is non-zero ───────────────────────────────────────────────────────────

#[test]
fn col_nonzero() {
    let diags = check("SELECT * FROM t ORDER BY a, b, c, d, e, f");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

// ── two statements each over max → 2 violations ──────────────────────────────

#[test]
fn two_statements_each_over_max_two_violations() {
    let sql = "SELECT * FROM t ORDER BY a, b, c, d, e, f; \
               SELECT * FROM u ORDER BY x, y, z, p, q, r";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

// ── subquery ORDER BY counted independently ───────────────────────────────────
// Outer ORDER BY has 3 columns (ok at default max=5).
// Inner subquery ORDER BY has 6 columns (flag).

#[test]
fn subquery_order_by_counted_independently() {
    let sql = "SELECT * FROM \
               (SELECT * FROM t ORDER BY a, b, c, d, e, f) AS sub \
               ORDER BY x, y, z";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
