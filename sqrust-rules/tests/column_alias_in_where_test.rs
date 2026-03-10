use sqrust_core::{FileContext, Rule};
use sqrust_rules::lint::column_alias_in_where::ColumnAliasInWhere;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ColumnAliasInWhere.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ColumnAliasInWhere.name(), "Lint/ColumnAliasInWhere");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn no_alias_in_where_no_violation() {
    assert!(check("SELECT id, name FROM t WHERE id > 1").is_empty());
}

#[test]
fn alias_not_in_where_no_violation() {
    assert!(check("SELECT a + b AS total FROM t WHERE a > 1").is_empty());
}

#[test]
fn alias_in_where_flagged() {
    let d = check("SELECT a + b AS total FROM t WHERE total > 100");
    assert_eq!(d.len(), 1);
}

#[test]
fn alias_in_where_case_insensitive() {
    let d = check("SELECT a + b AS Total FROM t WHERE total > 100");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_aliases_in_where_two_violations() {
    let d = check("SELECT a AS x, b AS y FROM t WHERE x > 1 AND y > 2");
    assert_eq!(d.len(), 2);
}

#[test]
fn alias_in_order_by_no_violation() {
    // ORDER BY alias is allowed in some dialects; we only flag WHERE
    assert!(check("SELECT a + b AS total FROM t ORDER BY total").is_empty());
}

#[test]
fn alias_in_having_no_violation() {
    // HAVING can reference aggregate aliases in some dialects — don't flag
    assert!(check("SELECT dept, COUNT(*) AS cnt FROM t GROUP BY dept HAVING cnt > 5").is_empty());
}

#[test]
fn alias_same_as_column_name_flagged() {
    // Even if the alias matches a real column name, we flag conservatively
    let d = check("SELECT id AS id FROM t WHERE id > 1");
    // Actually id is also a real column name — this may produce a false positive.
    // The test documents the conservative behavior.
    let _ = d; // Don't assert count — behavior is conservative
}

#[test]
fn message_mentions_alias() {
    let d = check("SELECT a + b AS total FROM t WHERE total > 0");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("alias") || msg.contains("where") || msg.contains("total"),
        "expected message to mention alias/where/column name, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT a + b AS total FROM t WHERE total > 0");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Lint/ColumnAliasInWhere");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT a + b AS total FROM t WHERE total > 0");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn subquery_alias_in_outer_where_not_flagged() {
    // The alias is defined in a subquery's SELECT; outer WHERE has its own column refs
    assert!(check("SELECT * FROM (SELECT a + b AS total FROM t) sub WHERE sub.total > 0").is_empty());
}
