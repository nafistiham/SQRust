use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::or_in_join_condition::OrInJoinCondition;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    OrInJoinCondition.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(OrInJoinCondition.name(), "Ambiguous/OrInJoinCondition");
}

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = OrInJoinCondition.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn join_with_simple_on_no_violation() {
    let diags = check("SELECT * FROM t JOIN u ON t.id = u.id");
    assert!(diags.is_empty());
}

#[test]
fn join_with_or_in_on_one_violation() {
    let diags = check("SELECT * FROM t JOIN u ON t.id = u.id OR t.code = u.code");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/OrInJoinCondition");
}

#[test]
fn join_with_and_in_on_no_violation() {
    let diags = check("SELECT * FROM t JOIN u ON t.id = u.id AND t.code = u.code");
    assert!(diags.is_empty());
}

#[test]
fn left_join_with_or_one_violation() {
    let diags = check("SELECT * FROM t LEFT JOIN u ON t.id = u.id OR t.name = u.name");
    assert_eq!(diags.len(), 1);
}

#[test]
fn cross_join_no_violation() {
    // CROSS JOIN has no ON clause at all
    let diags = check("SELECT * FROM t CROSS JOIN u");
    assert!(diags.is_empty());
}

#[test]
fn join_using_no_violation() {
    let diags = check("SELECT * FROM t JOIN u USING (id)");
    assert!(diags.is_empty());
}

#[test]
fn nested_or_in_on_one_violation() {
    // OR is wrapped in parentheses and combined with AND
    let diags = check("SELECT * FROM t JOIN u ON (t.a = u.a OR t.b = u.b) AND t.c = u.c");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_joins_only_one_has_or_one_violation() {
    let sql = "SELECT * FROM t \
               JOIN u ON t.id = u.id \
               LEFT JOIN v ON t.id = v.id OR t.name = v.name";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_joins_both_have_or_two_violations() {
    let sql = "SELECT * FROM t \
               JOIN u ON t.id = u.id OR t.code = u.code \
               LEFT JOIN v ON t.id = v.id OR t.name = v.name";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn no_join_no_violation() {
    let diags = check("SELECT * FROM t WHERE t.id = 1");
    assert!(diags.is_empty());
}

#[test]
fn message_format_is_correct() {
    let diags = check("SELECT * FROM t JOIN u ON t.id = u.id OR t.code = u.code");
    assert_eq!(diags.len(), 1);
    assert_eq!(
        diags[0].message,
        "OR condition in JOIN ON clause; this may produce unintended cross-join-like results"
    );
}

#[test]
fn line_and_col_are_nonzero() {
    let diags = check("SELECT * FROM t JOIN u ON t.id = u.id OR t.code = u.code");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn subquery_join_with_or_one_violation() {
    let sql = "SELECT * FROM (SELECT * FROM t JOIN u ON t.id = u.id OR t.x = u.x) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
