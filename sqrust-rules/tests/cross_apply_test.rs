use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::cross_apply::CrossApply;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CrossApply.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CrossApply.name(), "Structure/CrossApply");
}

#[test]
fn no_apply_no_violation() {
    let diags = check("SELECT o.id, p.name FROM orders o JOIN products p ON o.product_id = p.id");
    assert!(diags.is_empty());
}

#[test]
fn cross_apply_flagged() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o CROSS APPLY get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn outer_apply_flagged() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o OUTER APPLY get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn cross_apply_rule_name_is_correct() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o CROSS APPLY get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/CrossApply");
}

#[test]
fn outer_apply_rule_name_is_correct() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o OUTER APPLY get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/CrossApply");
}

#[test]
fn cross_apply_message_mentions_lateral_join() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o CROSS APPLY get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("lateral"),
        "expected CROSS APPLY message to mention LATERAL, got: {}",
        diags[0].message
    );
}

#[test]
fn outer_apply_message_mentions_lateral_join() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o OUTER APPLY get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("lateral"),
        "expected OUTER APPLY message to mention LATERAL, got: {}",
        diags[0].message
    );
}

#[test]
fn cross_apply_case_insensitive() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o cross apply get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1, "detection should be case-insensitive");
}

#[test]
fn outer_apply_case_insensitive() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o outer apply get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1, "detection should be case-insensitive");
}

#[test]
fn cross_apply_in_string_not_flagged() {
    // CROSS APPLY inside a string literal should not be flagged
    let diags = check("SELECT 'CROSS APPLY is SQL Server syntax' AS note FROM t");
    assert!(diags.is_empty(), "CROSS APPLY in string literal should not be flagged");
}

#[test]
fn outer_apply_in_comment_not_flagged() {
    // OUTER APPLY inside a line comment should not be flagged
    let sql = "-- Use OUTER APPLY if you want SQL Server syntax\nSELECT id FROM t";
    let diags = check(sql);
    assert!(diags.is_empty(), "OUTER APPLY in comment should not be flagged");
}

#[test]
fn multiple_apply_multiple_violations() {
    let diags = check(
        "SELECT a.id, b.val, c.val FROM a CROSS APPLY fn1(a.id) b OUTER APPLY fn2(a.id) c",
    );
    assert_eq!(diags.len(), 2, "each APPLY occurrence should produce one violation");
}

#[test]
fn line_col_nonzero_cross_apply() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o CROSS APPLY get_details(o.id) f",
    );
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn line_col_nonzero_outer_apply() {
    let diags = check(
        "SELECT o.id, f.val FROM orders o OUTER APPLY get_details(o.id) f",
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
fn word_apply_alone_not_flagged() {
    // A plain word "APPLY" without CROSS or OUTER prefix should not be flagged
    let diags = check("SELECT id FROM t WHERE apply_flag = 1");
    assert!(diags.is_empty(), "plain APPLY should not be flagged");
}
