use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::case_null_check::CaseNullCheck;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CaseNullCheck.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(CaseNullCheck.name(), "Ambiguous/CaseNullCheck");
}

#[test]
fn case_when_col_eq_null_one_violation() {
    let diags = check("SELECT CASE WHEN col = NULL THEN 1 ELSE 0 END FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/CaseNullCheck");
}

#[test]
fn case_when_col_is_null_no_violation() {
    let diags = check("SELECT CASE WHEN col IS NULL THEN 1 ELSE 0 END FROM t");
    assert!(diags.is_empty());
}

#[test]
fn case_when_col_noteq_null_one_violation() {
    let diags = check("SELECT CASE WHEN col <> NULL THEN 1 ELSE 0 END FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_when_col_bang_eq_null_one_violation() {
    // sqlparser may parse != as NotEq
    let diags = check("SELECT CASE WHEN col != NULL THEN 1 ELSE 0 END FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_when_col_eq_zero_no_violation() {
    let diags = check("SELECT CASE WHEN col = 0 THEN 1 ELSE 0 END FROM t");
    assert!(diags.is_empty());
}

#[test]
fn two_null_comparisons_in_case_two_violations() {
    let diags = check(
        "SELECT CASE WHEN a = NULL THEN 1 WHEN b = NULL THEN 2 ELSE 0 END FROM t",
    );
    assert_eq!(diags.len(), 2);
}

#[test]
fn one_null_one_is_null_one_violation() {
    let diags = check(
        "SELECT CASE WHEN a = NULL THEN 1 WHEN b IS NULL THEN 2 ELSE 0 END FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn nested_case_null_check_one_violation() {
    let diags = check(
        "SELECT CASE WHEN x = 1 THEN CASE WHEN y = NULL THEN 'a' ELSE 'b' END ELSE 'c' END FROM t",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_in_where_clause_one_violation() {
    let diags = check(
        "SELECT a FROM t WHERE CASE WHEN col = NULL THEN 1 ELSE 0 END = 1",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn case_operand_when_null_one_violation() {
    // CASE col WHEN NULL THEN 1 ELSE 0 END — comparing operand=NULL is also always false
    let diags = check("SELECT CASE col WHEN NULL THEN 1 ELSE 0 END FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn plain_where_col_eq_null_no_violation() {
    // This is not inside a CASE — should be handled by is_null.rs, not this rule
    let diags = check("SELECT a FROM t WHERE col = NULL");
    assert!(diags.is_empty());
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = CaseNullCheck.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn string_null_not_null_value_no_violation() {
    let diags = check("SELECT CASE WHEN col = 'NULL' THEN 1 ELSE 0 END FROM t");
    assert!(diags.is_empty());
}
