use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::excessive_group_by_columns::ExcessiveGroupByColumns;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    ExcessiveGroupByColumns::default().check(&c)
}

fn check_with(sql: &str, max_columns: usize) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    ExcessiveGroupByColumns { max_columns }.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        ExcessiveGroupByColumns::default().name(),
        "ExcessiveGroupByColumns"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("SELECT FROM GROUP BY BROKEN ,,,,");
    assert!(diags.is_empty());
}

// ── default max is 5 ─────────────────────────────────────────────────────────

#[test]
fn default_max_is_five() {
    assert_eq!(ExcessiveGroupByColumns::default().max_columns, 5);
}

// ── 5 columns at max — no violation ──────────────────────────────────────────

#[test]
fn five_columns_at_max_no_violation() {
    let diags = check("SELECT a, b, c, d, e FROM t GROUP BY a, b, c, d, e");
    assert!(diags.is_empty());
}

// ── 6 columns over max — 1 violation ─────────────────────────────────────────

#[test]
fn six_columns_over_max_one_violation() {
    let diags = check("SELECT a, b, c, d, e, f FROM t GROUP BY a, b, c, d, e, f");
    assert_eq!(diags.len(), 1);
}

// ── 3 columns under max — no violation ───────────────────────────────────────

#[test]
fn three_columns_under_max_no_violation() {
    let diags = check("SELECT a, b, c FROM t GROUP BY a, b, c");
    assert!(diags.is_empty());
}

// ── custom max 3 with 4 columns — 1 violation ─────────────────────────────────

#[test]
fn custom_max_3_with_4_columns_one_violation() {
    let diags = check_with("SELECT a, b, c, d FROM t GROUP BY a, b, c, d", 3);
    assert_eq!(diags.len(), 1);
}

// ── custom max 3 with 3 columns — no violation ────────────────────────────────

#[test]
fn custom_max_3_with_3_columns_no_violation() {
    let diags = check_with("SELECT a, b, c FROM t GROUP BY a, b, c", 3);
    assert!(diags.is_empty());
}

// ── no GROUP BY — no violation ────────────────────────────────────────────────

#[test]
fn no_group_by_no_violation() {
    let diags = check("SELECT id FROM t");
    assert!(diags.is_empty());
}

// ── message contains count and max ────────────────────────────────────────────

#[test]
fn message_contains_count_and_max() {
    let diags = check("SELECT a, b, c, d, e, f FROM t GROUP BY a, b, c, d, e, f");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('6'),
        "message should contain the actual column count (6)"
    );
    assert!(
        diags[0].message.contains('5'),
        "message should contain the maximum (5)"
    );
}

// ── line/col nonzero ─────────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT a, b, c, d, e, f FROM t GROUP BY a, b, c, d, e, f");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── two selects both excessive — 2 violations ─────────────────────────────────

#[test]
fn two_selects_both_excessive_two_violations() {
    let sql = "SELECT a, b, c, d, e, f FROM t GROUP BY a, b, c, d, e, f; \
               SELECT a, b, c, d, e, f FROM t GROUP BY a, b, c, d, e, f";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

// ── GROUP BY ALL — handle gracefully (0 violations) ──────────────────────────

#[test]
fn group_by_all_no_violation() {
    // GROUP BY ALL is a DuckDB/Snowflake extension; the AST uses GroupByExpr::All.
    // GenericDialect may or may not parse it. If it fails to parse, the rule
    // returns 0 anyway. Either way expect no ExcessiveGroupByColumns violation.
    let diags = check("SELECT a, b, c FROM t GROUP BY ALL");
    // Either 0 violations (parsed as All) or 0 violations (parse error).
    assert!(diags.is_empty());
}

// ── subquery with excessive GROUP BY — 1 violation ───────────────────────────

#[test]
fn subquery_with_excessive_group_by_violation() {
    let sql = "SELECT * FROM (SELECT a, b, c, d, e, f FROM t GROUP BY a, b, c, d, e, f) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
