use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::mixed_join_types::MixedJoinTypes;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    MixedJoinTypes.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(MixedJoinTypes.name(), "Ambiguous/MixedJoinTypes");
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = MixedJoinTypes.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn inner_and_left_join_one_violation() {
    let sql = "SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id LEFT JOIN t3 ON t1.id = t3.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn inner_and_right_join_one_violation() {
    let sql = "SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id RIGHT JOIN t3 ON t1.id = t3.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn inner_and_full_outer_join_one_violation() {
    let sql = "SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id FULL OUTER JOIN t3 ON t1.id = t3.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn only_inner_joins_no_violation() {
    let sql = "SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id INNER JOIN t3 ON t1.id = t3.id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn only_left_joins_no_violation() {
    let sql = "SELECT * FROM t1 LEFT JOIN t2 ON t1.id = t2.id LEFT JOIN t3 ON t1.id = t3.id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn only_right_joins_no_violation() {
    let sql = "SELECT * FROM t1 RIGHT JOIN t2 ON t1.id = t2.id RIGHT JOIN t3 ON t1.id = t3.id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn cross_join_and_left_join_one_violation() {
    // CROSS JOIN counts as inner for this rule's purposes
    let sql = "SELECT * FROM t1 CROSS JOIN t2 LEFT JOIN t3 ON t1.id = t3.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_inner_one_left_one_violation() {
    let sql = concat!(
        "SELECT * FROM t1 ",
        "INNER JOIN t2 ON t1.id = t2.id ",
        "INNER JOIN t3 ON t1.id = t3.id ",
        "INNER JOIN t4 ON t1.id = t4.id ",
        "LEFT JOIN t5 ON t1.id = t5.id"
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn no_joins_no_violation() {
    let diags = check("SELECT * FROM t1");
    assert!(diags.is_empty());
}

#[test]
fn message_contains_join() {
    let sql = "SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id LEFT JOIN t3 ON t1.id = t3.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.to_ascii_uppercase().contains("JOIN"),
        "message should mention JOIN, got: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "SELECT * FROM t1 INNER JOIN t2 ON t1.id = t2.id LEFT JOIN t3 ON t1.id = t3.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn subquery_with_mixed_joins_violation() {
    // Mixing in a subquery is also flagged
    let sql = concat!(
        "SELECT * FROM (",
        "  SELECT * FROM t1 ",
        "  INNER JOIN t2 ON t1.id = t2.id ",
        "  LEFT JOIN t3 ON t1.id = t3.id",
        ") sub"
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
