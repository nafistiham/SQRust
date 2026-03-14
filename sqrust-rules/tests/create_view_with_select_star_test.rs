use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::create_view_with_select_star::CreateViewWithSelectStar;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    CreateViewWithSelectStar.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        CreateViewWithSelectStar.name(),
        "Lint/CreateViewWithSelectStar"
    );
}

#[test]
fn create_view_select_star_violation() {
    let sql = "CREATE VIEW v AS SELECT * FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_view_select_columns_no_violation() {
    let sql = "CREATE VIEW v AS SELECT id, name FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_or_replace_view_select_star_violation() {
    let sql = "CREATE OR REPLACE VIEW v AS SELECT * FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_or_replace_view_select_columns_no_violation() {
    let sql = "CREATE OR REPLACE VIEW v AS SELECT id, name FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_view_lowercase_select_star_violation() {
    let sql = "create view v as select * from t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_view_mixed_case_select_star_violation() {
    let sql = "Create View v As Select * From t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn select_star_outside_view_no_violation() {
    let sql = "SELECT * FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn create_table_with_select_star_no_violation() {
    // CREATE TABLE ... AS SELECT * is different rule territory
    let sql = "CREATE TABLE t AS SELECT * FROM source";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_select_star() {
    let sql = "CREATE VIEW v AS SELECT * FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let lower = diags[0].message.to_lowercase();
    assert!(
        lower.contains("select *"),
        "message should mention SELECT *: {}",
        diags[0].message
    );
}

#[test]
fn message_mentions_fragile() {
    let sql = "CREATE VIEW v AS SELECT * FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let lower = diags[0].message.to_lowercase();
    assert!(
        lower.contains("fragile") || lower.contains("column"),
        "message should explain why it's problematic: {}",
        diags[0].message
    );
}

#[test]
fn line_col_nonzero() {
    let sql = "CREATE VIEW v AS SELECT * FROM t";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn violation_on_second_line_has_correct_line() {
    let sql = concat!("SELECT 1;\n", "CREATE VIEW v AS SELECT * FROM t");
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn skip_create_view_select_star_in_string_literal() {
    let sql = "SELECT 'create view v as select * from t' FROM t";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn skip_create_view_select_star_in_line_comment() {
    let sql = "-- create view v as select * from t\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn works_with_parse_error_source_level_scan() {
    let sql = "CREATE VIEW v AS SELECT * FROM t ### invalid";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_views_with_select_star_two_violations() {
    let sql = concat!(
        "CREATE VIEW v1 AS SELECT * FROM a;\n",
        "CREATE VIEW v2 AS SELECT * FROM b",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}
