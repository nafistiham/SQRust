use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::subquery_in_join_condition::SubqueryInJoinCondition;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SubqueryInJoinCondition.check(&ctx)
}

// ── rule metadata ─────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(SubqueryInJoinCondition.name(), "Structure/SubqueryInJoinCondition");
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SubqueryInJoinCondition.check(&ctx);
        assert!(diags.is_empty());
    }
}

// ── violations ────────────────────────────────────────────────────────────────

#[test]
fn subquery_directly_in_on_clause_one_violation() {
    let sql =
        "SELECT a.id FROM a JOIN b ON (SELECT MAX(id) FROM c) = a.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/SubqueryInJoinCondition");
}

#[test]
fn subquery_in_on_clause_multiline_one_violation() {
    let sql = "SELECT a.id\nFROM a\nJOIN b ON\n(SELECT MAX(id) FROM c) = a.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn subquery_in_on_clause_indented_one_violation() {
    let sql = "SELECT a.id\nFROM a\nJOIN b ON\n    (SELECT MAX(id) FROM c) = a.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn subquery_in_on_clause_lowercase_one_violation() {
    let sql = "select a.id from a join b on (select max(id) from c) = a.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_joins_with_subqueries_two_violations() {
    let sql = "SELECT a.id FROM a \
               JOIN b ON (SELECT MAX(id) FROM c) = a.id \
               JOIN d ON (SELECT MIN(id) FROM e) = a.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

// ── no violations ─────────────────────────────────────────────────────────────

#[test]
fn simple_join_no_violation() {
    let sql = "SELECT a.id FROM a JOIN b ON a.id = b.id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn join_on_with_column_comparison_no_violation() {
    let sql = "SELECT * FROM orders o JOIN customers c ON o.customer_id = c.id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn subquery_in_where_not_flagged() {
    let sql = "SELECT a.id FROM a JOIN b ON a.id = b.id WHERE a.id IN (SELECT id FROM c)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn subquery_in_select_not_flagged() {
    let sql = "SELECT (SELECT MAX(id) FROM b) AS max_id FROM a JOIN c ON a.id = c.a_id";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn no_join_no_violation() {
    let sql = "SELECT id FROM t WHERE id > 5";
    let diags = check(sql);
    assert!(diags.is_empty());
}

// ── diagnostic fields ─────────────────────────────────────────────────────────

#[test]
fn message_contains_expected_text() {
    let sql = "SELECT a.id FROM a JOIN b ON (SELECT MAX(id) FROM c) = a.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("index") || msg.contains("CTE") || msg.contains("subquery") || msg.contains("JOIN"),
        "unexpected message: {msg}"
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "SELECT a.id FROM a JOIN b ON (SELECT MAX(id) FROM c) = a.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1, "line must be >= 1");
    assert!(diags[0].col >= 1, "col must be >= 1");
}
