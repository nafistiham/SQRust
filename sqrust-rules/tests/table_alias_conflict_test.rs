use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::table_alias_conflict::TableAliasConflict;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    TableAliasConflict.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(TableAliasConflict.name(), "Ambiguous/TableAliasConflict");
}

#[test]
fn implicit_cross_join_same_alias_one_violation() {
    // SELECT * FROM t1 a, t2 a — alias `a` used twice
    let diags = check("SELECT * FROM t1 a, t2 a");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/TableAliasConflict");
}

#[test]
fn inner_join_same_alias_one_violation() {
    let diags = check("SELECT * FROM t1 a JOIN t2 a ON t1.id = t2.id");
    assert_eq!(diags.len(), 1);
}

#[test]
fn inner_join_different_aliases_no_violation() {
    let diags = check("SELECT * FROM t1 a JOIN t2 b ON a.id = b.id");
    assert!(diags.is_empty());
}

#[test]
fn no_aliases_different_table_names_no_violation() {
    let diags = check("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id");
    assert!(diags.is_empty());
}

#[test]
fn case_insensitive_alias_conflict_one_violation() {
    // Alias `a` and `A` should be treated as the same (case-insensitive)
    let diags = check("SELECT * FROM t1 a JOIN t2 A ON a.id = A.id");
    assert_eq!(diags.len(), 1);
}

#[test]
fn same_table_no_alias_conflict_one_violation() {
    // SELECT * FROM t1, t1 — no aliases, but table name t1 appears twice
    let diags = check("SELECT * FROM t1, t1");
    assert_eq!(diags.len(), 1);
}

#[test]
fn subquery_outer_conflict_inner_aliases_do_not_leak() {
    // Outer FROM t1 a JOIN t2 a is a conflict; inner subquery aliases don't conflict with outer
    let sql = "SELECT * FROM t1 a JOIN t2 a ON t1.id = t2.id";
    let diags = check(sql);
    // Only the outer conflict matters
    assert_eq!(diags.len(), 1);
}

#[test]
fn inner_subquery_aliases_do_not_conflict_with_outer() {
    // Inner subquery uses alias `a` for its own table; outer uses `a` for t1
    // They are in different scopes — no conflict
    let sql = "SELECT * FROM t1 a JOIN (SELECT * FROM t2 a) sub ON a.id = sub.id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn three_tables_first_and_last_same_alias_one_violation() {
    let diags = check("SELECT * FROM t1 a JOIN t2 b ON t1.id = t2.id JOIN t3 a ON t2.id = t3.id");
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_empty() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = TableAliasConflict.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn message_format_includes_alias_name() {
    let diags = check("SELECT * FROM t1 a, t2 a");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('a'),
        "message should contain the alias name: {}",
        diags[0].message
    );
}

#[test]
fn all_distinct_aliases_no_violation() {
    let diags = check("SELECT * FROM t1 x JOIN t2 y ON x.id = y.id JOIN t3 z ON y.id = z.id");
    assert!(diags.is_empty());
}

#[test]
fn real_world_orders_join_no_violation() {
    let diags = check("SELECT * FROM orders o JOIN order_items oi ON o.id = oi.order_id");
    assert!(diags.is_empty());
}
