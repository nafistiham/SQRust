use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::self_join::SelfJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SelfJoin.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelfJoin.name(), "Ambiguous/SelfJoin");
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SelfJoin.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn no_self_join_no_violation() {
    let diags = check("SELECT * FROM t JOIN u ON t.id = u.id");
    assert!(diags.is_empty());
}

#[test]
fn self_join_no_aliases_one_violation() {
    let diags = check("SELECT * FROM t JOIN t ON t.id = t.id");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/SelfJoin");
}

#[test]
fn self_join_same_alias_one_violation() {
    let diags = check("SELECT * FROM t a JOIN t a ON a.id = a.id");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/SelfJoin");
}

#[test]
fn self_join_distinct_aliases_no_violation() {
    let diags = check("SELECT * FROM t a JOIN t b ON a.id = b.id");
    assert!(diags.is_empty());
}

#[test]
fn self_join_one_alias_one_without_flagged() {
    // One occurrence has no alias, one has an alias — still ambiguous
    let diags = check("SELECT * FROM t JOIN t a ON t.id = a.id");
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_tables_no_self_join_no_violation() {
    let diags = check("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id JOIN t3 ON t2.id = t3.id");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_table_name() {
    let diags = check("SELECT * FROM orders JOIN orders ON orders.id = orders.id");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("orders"),
        "expected message to contain 'orders', got: {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let diags = check("SELECT * FROM t JOIN t ON t.id = t.id");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn col_nonzero() {
    let diags = check("SELECT * FROM t JOIN t ON t.id = t.id");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn lowercase_table_names_flagged() {
    let diags = check("select * from orders join orders on orders.id = orders.id");
    assert_eq!(diags.len(), 1);
}

#[test]
fn self_join_in_subquery_flagged() {
    let sql = "SELECT * FROM (SELECT * FROM t JOIN t ON t.id = t.id) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_different_self_joins_two_violations() {
    // Two separate queries, each with a self join
    let sql = "SELECT * FROM t JOIN t ON t.id = t.id; SELECT * FROM u JOIN u ON u.id = u.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}
