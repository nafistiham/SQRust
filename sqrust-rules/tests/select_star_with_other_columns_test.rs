use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::ambiguous::select_star_with_other_columns::SelectStarWithOtherColumns;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    SelectStarWithOtherColumns.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        SelectStarWithOtherColumns.name(),
        "Ambiguous/SelectStarWithOtherColumns"
    );
}

#[test]
fn star_then_column_one_violation() {
    let diags = check("SELECT *, col1 FROM t");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Ambiguous/SelectStarWithOtherColumns");
}

#[test]
fn column_then_star_one_violation() {
    let diags = check("SELECT col1, * FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn star_alone_no_violation() {
    let diags = check("SELECT * FROM t");
    assert!(diags.is_empty());
}

#[test]
fn explicit_columns_only_no_violation() {
    let diags = check("SELECT col1, col2 FROM t");
    assert!(diags.is_empty());
}

#[test]
fn qualified_wildcard_only_no_violation() {
    // t.* is a qualified wildcard — no explicit column references, no mixing
    let diags = check("SELECT t.* FROM t");
    assert!(diags.is_empty());
}

#[test]
fn qualified_wildcard_with_explicit_column_one_violation() {
    // t.* mixed with col1 — still ambiguous
    let diags = check("SELECT t.*, col1 FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn all_stars_no_violation() {
    // SELECT *, * — both are wildcards, no explicit columns
    let diags = check("SELECT *, * FROM t");
    assert!(diags.is_empty());
}

#[test]
fn star_in_middle_one_violation() {
    let diags = check("SELECT col1, *, col2 FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn subquery_with_mixing_detected() {
    // The inner SELECT has *, col1 — should be flagged
    let sql = "SELECT id FROM (SELECT *, col1 FROM t) sub";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn outer_clean_inner_clean_no_violation() {
    let sql = "SELECT id FROM (SELECT col1, col2 FROM t) sub";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn union_both_clean_no_violation() {
    let sql = "SELECT col1, col2 FROM t UNION ALL SELECT col3, col4 FROM u";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn union_one_side_violates() {
    let sql = "SELECT col1, col2 FROM t UNION ALL SELECT *, col1 FROM u";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn parse_error_returns_no_violations() {
    let sql = "SELECTT INVALID GARBAGE @@##";
    let ctx = FileContext::from_source(sql, "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = SelectStarWithOtherColumns.check(&ctx);
        assert!(diags.is_empty());
    }
}

#[test]
fn correct_message_text() {
    let diags = check("SELECT *, col1 FROM t");
    assert_eq!(
        diags[0].message,
        "Avoid mixing SELECT * with explicit columns; either use * alone or list all columns explicitly"
    );
}
