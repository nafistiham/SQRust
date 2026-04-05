use sqrust_core::{FileContext, Rule};
use sqrust_rules::structure::multiple_statements_in_file::MultipleStatementsInFile;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    MultipleStatementsInFile.check(&ctx(sql))
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        MultipleStatementsInFile.name(),
        "Structure/MultipleStatementsInFile"
    );
}

// ── single statements — no violation ─────────────────────────────────────────

#[test]
fn single_select_no_violation() {
    let diags = check("SELECT 1");
    assert!(diags.is_empty());
}

#[test]
fn single_cte_query_no_violation() {
    let diags = check("WITH cte AS (SELECT 1) SELECT * FROM cte");
    assert!(diags.is_empty());
}

#[test]
fn single_insert_no_violation() {
    let diags = check("INSERT INTO t VALUES (1)");
    assert!(diags.is_empty());
}

#[test]
fn select_with_trailing_semicolon_no_violation() {
    // A trailing semicolon should still count as a single statement.
    let diags = check("SELECT 1;");
    assert!(diags.is_empty());
}

#[test]
fn empty_file_no_violation() {
    let diags = check("");
    assert!(diags.is_empty());
}

// ── multiple statements — violation ──────────────────────────────────────────

#[test]
fn two_selects_violation() {
    let diags = check("SELECT 1; SELECT 2");
    assert_eq!(diags.len(), 1);
}

#[test]
fn three_selects_violation() {
    let diags = check("SELECT 1; SELECT 2; SELECT 3");
    assert_eq!(diags.len(), 1);
}

#[test]
fn create_and_select_violation() {
    let diags = check("CREATE TABLE t (id INT); SELECT * FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_inserts_violation() {
    let diags = check("INSERT INTO t VALUES (1); INSERT INTO t VALUES (2)");
    assert_eq!(diags.len(), 1);
}

// ── diagnostic content ────────────────────────────────────────────────────────

#[test]
fn violation_message_contains_count() {
    let diags = check("SELECT 1; SELECT 2; SELECT 3");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains('3'),
        "message should contain the statement count (3); got: {}",
        diags[0].message
    );
}

#[test]
fn violation_at_line_1() {
    let diags = check("SELECT 1; SELECT 2");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 1);
}

#[test]
fn violation_col_is_1() {
    let diags = check("SELECT 1; SELECT 2");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 1);
}

// ── parse error — no violation ────────────────────────────────────────────────

#[test]
fn parse_error_no_violation() {
    // Malformed SQL that fails to parse should return no diagnostics.
    let sql = "SELECTT INVALID GARBAGE @@##";
    let context = ctx(sql);
    if !context.parse_errors.is_empty() {
        let diags = MultipleStatementsInFile.check(&context);
        assert!(
            diags.is_empty(),
            "expected no diagnostics on parse error; got: {} violations",
            diags.len()
        );
    }
}
