use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::insert_select_star::InsertSelectStar;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    InsertSelectStar.check(&c)
}

// ── rule name ────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(InsertSelectStar.name(), "Structure/InsertSelectStar");
}

// ── INSERT SELECT * — 1 violation ────────────────────────────────────────────

#[test]
fn insert_select_star_one_violation() {
    let diags = check("INSERT INTO t SELECT * FROM s");
    assert_eq!(diags.len(), 1);
}

// ── INSERT SELECT explicit columns — 0 violations ────────────────────────────

#[test]
fn insert_select_explicit_columns_no_violation() {
    let diags = check("INSERT INTO t SELECT a, b FROM s");
    assert!(diags.is_empty());
}

// ── INSERT SELECT qualified wildcard s.* — 1 violation ───────────────────────

#[test]
fn insert_select_qualified_wildcard_one_violation() {
    let diags = check("INSERT INTO t SELECT s.* FROM s");
    assert_eq!(diags.len(), 1);
}

// ── INSERT with column list + SELECT explicit — 0 violations ─────────────────

#[test]
fn insert_with_column_list_explicit_select_no_violation() {
    let diags = check("INSERT INTO t (a, b) SELECT a, b FROM s");
    assert!(diags.is_empty());
}

// ── INSERT with column list + SELECT * — 1 violation ─────────────────────────

#[test]
fn insert_with_column_list_select_star_one_violation() {
    let diags = check("INSERT INTO t (a, b) SELECT * FROM s");
    assert_eq!(diags.len(), 1);
}

// ── plain SELECT * not in INSERT — 0 violations ───────────────────────────────

#[test]
fn plain_select_star_no_violation() {
    let diags = check("SELECT * FROM t");
    assert!(diags.is_empty());
}

// ── INSERT SELECT with WHERE — no wildcard — 0 violations ────────────────────

#[test]
fn insert_select_with_where_no_wildcard_no_violation() {
    let diags = check("INSERT INTO t SELECT a, b, c FROM s WHERE id > 0");
    assert!(diags.is_empty());
}

// ── INSERT VALUES — 0 violations ─────────────────────────────────────────────

#[test]
fn insert_values_no_violation() {
    let diags = check("INSERT INTO t VALUES (1, 2)");
    assert!(diags.is_empty());
}

// ── parse error — 0 violations ───────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("INSERT INTO BROKEN SELECT FROM");
    assert!(diags.is_empty());
}

// ── multiple inserts: one with star, one without — 1 violation ───────────────

#[test]
fn multiple_inserts_one_star_one_violation() {
    let diags = check(
        "INSERT INTO t1 SELECT * FROM s1; \
         INSERT INTO t2 SELECT a, b FROM s2",
    );
    assert_eq!(diags.len(), 1);
}

// ── INSERT SELECT with qualified column references — 0 violations ─────────────

#[test]
fn insert_select_qualified_columns_no_violation() {
    let diags = check("INSERT INTO t SELECT t2.a, t2.b FROM t2");
    assert!(diags.is_empty());
}

// ── subquery in SELECT list body — outer projection not wildcard — 0 violations

#[test]
fn subquery_in_select_list_outer_not_wildcard_no_violation() {
    let diags = check(
        "INSERT INTO t SELECT a, (SELECT MAX(x) FROM s2) FROM s",
    );
    assert!(diags.is_empty());
}

// ── CTE + INSERT outer SELECT has no wildcard — 0 violations ─────────────────

#[test]
fn cte_insert_outer_select_no_wildcard_no_violation() {
    let diags = check(
        "INSERT INTO t WITH c AS (SELECT * FROM s) SELECT a FROM c",
    );
    assert!(diags.is_empty());
}

// ── message content ───────────────────────────────────────────────────────────

#[test]
fn diagnostic_message_mentions_fragile() {
    let diags = check("INSERT INTO t SELECT * FROM s");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("fragile") || diags[0].message.contains("SELECT *"),
        "message should mention fragility or SELECT *"
    );
}

// ── diagnostic has correct rule name ─────────────────────────────────────────

#[test]
fn diagnostic_has_correct_rule_name() {
    let diags = check("INSERT INTO t SELECT * FROM s");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/InsertSelectStar");
}
