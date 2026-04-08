use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::union_column_alias::UnionColumnAlias;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    UnionColumnAlias.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnionColumnAlias.name(), "Structure/UnionColumnAlias");
}

// ── alias in non-first UNION branch ──────────────────────────────────────────

#[test]
fn alias_in_second_union_branch_violation() {
    // 'y' is defined in the second branch — it is silently ignored by the DB
    let diags = check(
        "SELECT a AS x FROM t1 UNION SELECT b AS y FROM t2",
    );
    assert_eq!(diags.len(), 1);
}

// ── alias only in first branch — no violation ─────────────────────────────────

#[test]
fn alias_only_in_first_branch_no_violation() {
    let diags = check(
        "SELECT a AS x FROM t1 UNION SELECT b FROM t2",
    );
    assert!(diags.is_empty());
}

// ── no aliases anywhere ───────────────────────────────────────────────────────

#[test]
fn no_aliases_no_violation() {
    let diags = check(
        "SELECT a FROM t1 UNION SELECT b FROM t2",
    );
    assert!(diags.is_empty());
}

// ── UNION ALL also triggers ───────────────────────────────────────────────────

#[test]
fn union_all_violation() {
    let diags = check(
        "SELECT a FROM t1 UNION ALL SELECT b AS alias FROM t2",
    );
    assert_eq!(diags.len(), 1);
}

// ── three-way UNION — second and third branches both have aliases ─────────────

#[test]
fn three_way_union_violation() {
    let diags = check(
        "SELECT a FROM t1 UNION SELECT b AS y FROM t2 UNION SELECT c AS z FROM t3",
    );
    // Both second and third branches have aliases — expect 2 violations.
    assert_eq!(diags.len(), 2);
}

// ── empty file ────────────────────────────────────────────────────────────────

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

// ── parse error returns no violations ─────────────────────────────────────────

#[test]
fn parse_error_no_violation() {
    let c = ctx("UNION ALL SELECT FROM WHERE ORDER BY BROKEN ##");
    if !c.parse_errors.is_empty() {
        let diags = UnionColumnAlias.check(&c);
        assert!(diags.is_empty());
    }
}

// ── INTERSECT — do not flag ───────────────────────────────────────────────────

#[test]
fn intersect_no_violation() {
    let diags = check(
        "SELECT a FROM t1 INTERSECT SELECT b AS y FROM t2",
    );
    assert!(diags.is_empty());
}

// ── EXCEPT — do not flag ──────────────────────────────────────────────────────

#[test]
fn except_no_violation() {
    let diags = check(
        "SELECT a FROM t1 EXCEPT SELECT b AS y FROM t2",
    );
    assert!(diags.is_empty());
}

// ── alias in both first and second branches ───────────────────────────────────

#[test]
fn alias_in_both_branches_violation() {
    // First branch alias is fine, second branch alias is flagged.
    let diags = check(
        "SELECT a AS x FROM t1 UNION SELECT b AS y FROM t2",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("ignored"));
}

// ── no UNION — plain SELECT ───────────────────────────────────────────────────

#[test]
fn no_union_no_violation() {
    let diags = check(
        "SELECT a AS x, b AS y FROM t",
    );
    assert!(diags.is_empty());
}

// ── first branch alias — no violation ────────────────────────────────────────

#[test]
fn first_branch_alias_no_violation() {
    let diags = check(
        "SELECT a AS col1, b AS col2 FROM t1 UNION SELECT c, d FROM t2",
    );
    assert!(diags.is_empty());
}

// ── multiple aliases in single non-first branch ───────────────────────────────

#[test]
fn multiple_aliases_in_non_first_branch() {
    let diags = check(
        "SELECT a, b FROM t1 UNION SELECT c AS x, d AS y FROM t2",
    );
    // Two aliases in the second branch — two violations.
    assert_eq!(diags.len(), 2);
}
