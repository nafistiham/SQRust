use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::exists_select_list::ExistsSelectList;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    ExistsSelectList.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ExistsSelectList.name(), "Ambiguous/ExistsSelectList");
}

#[test]
fn exists_with_column_ref_one_violation() {
    let diags = check("SELECT * FROM t WHERE EXISTS (SELECT col FROM s WHERE s.id = t.id)");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/ExistsSelectList");
}

#[test]
fn exists_with_select_1_no_violation() {
    let diags = check("SELECT * FROM t WHERE EXISTS (SELECT 1 FROM s WHERE s.id = t.id)");
    assert!(diags.is_empty());
}

#[test]
fn exists_with_select_star_no_violation() {
    let diags = check("SELECT * FROM t WHERE EXISTS (SELECT * FROM s WHERE s.id = t.id)");
    assert!(diags.is_empty());
}

#[test]
fn exists_with_select_null_no_violation() {
    let diags = check("SELECT * FROM t WHERE EXISTS (SELECT NULL FROM s WHERE s.id = t.id)");
    assert!(diags.is_empty());
}

#[test]
fn exists_with_multiple_columns_one_violation() {
    let diags = check("SELECT * FROM t WHERE EXISTS (SELECT a, b FROM s WHERE s.id = t.id)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_exists_with_column_ref_one_violation() {
    let diags = check("SELECT * FROM t WHERE NOT EXISTS (SELECT col FROM s WHERE s.id = t.id)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn exists_with_string_literal_no_violation() {
    let diags = check("SELECT * FROM t WHERE EXISTS (SELECT 'x' FROM s)");
    assert!(diags.is_empty());
}

#[test]
fn exists_in_case_expression_one_violation() {
    let diags = check(
        "SELECT CASE WHEN EXISTS (SELECT col FROM s WHERE s.id = t.id) THEN 1 ELSE 0 END FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn exists_in_subquery_one_violation() {
    let diags = check(
        "SELECT a FROM (SELECT * FROM t WHERE EXISTS (SELECT col FROM s WHERE s.id = t.id)) sub",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn exists_in_cte_one_violation() {
    let diags = check(
        "WITH c AS (SELECT * FROM t WHERE EXISTS (SELECT col FROM s)) SELECT * FROM c",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = ExistsSelectList.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn exists_with_qualified_col_ref_one_violation() {
    let diags =
        check("SELECT * FROM t WHERE EXISTS (SELECT s.id FROM s WHERE s.id = t.id)");
    assert_eq!(diags.len(), 1);
}

#[test]
fn message_contains_exists_or_select_1() {
    let diags = check("SELECT * FROM t WHERE EXISTS (SELECT col FROM s WHERE s.id = t.id)");
    assert_eq!(diags.len(), 1);
    let msg = &diags[0].message;
    assert!(
        msg.contains("EXISTS") || msg.contains("SELECT 1"),
        "message was: {msg}"
    );
}
