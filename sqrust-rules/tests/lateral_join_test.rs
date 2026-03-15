use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::lateral_join::LateralJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    LateralJoin.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LateralJoin.name(), "Structure/LateralJoin");
}

#[test]
fn no_lateral_no_violation() {
    let diags = check("SELECT o.id, p.name FROM orders o JOIN products p ON o.product_id = p.id");
    assert!(diags.is_empty());
}

#[test]
fn lateral_join_flagged() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o, LATERAL get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn lateral_subquery_flagged() {
    let diags = check(
        "SELECT a.id, b.total FROM a JOIN LATERAL (SELECT SUM(val) AS total FROM b WHERE b.a_id = a.id) sub ON TRUE",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn rule_name_in_diagnostic() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o, LATERAL get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/LateralJoin");
}

#[test]
fn message_mentions_sql_server_or_unsupported() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o, LATERAL get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("sql server") || msg.contains("unsupported") || msg.contains("not supported"),
        "expected message to mention SQL Server or unsupported, got: {}",
        diags[0].message
    );
}

#[test]
fn lateral_case_insensitive_lower() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o, lateral get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1, "detection should be case-insensitive");
}

#[test]
fn lateral_case_insensitive_mixed() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o, Lateral get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1, "detection should be case-insensitive");
}

#[test]
fn lateral_in_string_not_flagged() {
    let diags = check("SELECT 'LATERAL joins are not supported everywhere' AS note FROM t");
    assert!(diags.is_empty(), "LATERAL in string literal should not be flagged");
}

#[test]
fn lateral_in_line_comment_not_flagged() {
    let sql = "-- Use LATERAL for PostgreSQL or MySQL 8.0+\nSELECT id FROM t";
    let diags = check(sql);
    assert!(diags.is_empty(), "LATERAL in line comment should not be flagged");
}

#[test]
fn lateral_in_block_comment_not_flagged() {
    let sql = "/* LATERAL join example */\nSELECT id FROM t";
    let diags = check(sql);
    assert!(diags.is_empty(), "LATERAL in block comment should not be flagged");
}

#[test]
fn multiple_lateral_multiple_violations() {
    let diags = check(
        "SELECT a.id, b.x, c.y FROM a, LATERAL fn1(a.id) b, LATERAL fn2(a.id) c",
    );
    assert_eq!(diags.len(), 2, "each LATERAL occurrence should produce one violation");
}

#[test]
fn line_col_nonzero() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o, LATERAL get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn empty_sql_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn lateral_column_alias_word_boundary_not_flagged() {
    // "laterally" should not be flagged — must be exact word boundary
    let diags = check("SELECT id FROM t WHERE laterally_ordered = 1");
    assert!(diags.is_empty(), "word containing LATERAL as prefix should not be flagged");
}

#[test]
fn lateral_join_with_left_keyword() {
    let diags = check(
        "SELECT a.id, b.val FROM a LEFT JOIN LATERAL (SELECT val FROM b WHERE b.aid = a.id LIMIT 1) b ON TRUE",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn plain_select_no_lateral_no_violation() {
    let diags = check("SELECT id, name FROM users WHERE active = 1 ORDER BY name");
    assert!(diags.is_empty());
}
