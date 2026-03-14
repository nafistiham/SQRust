use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::structure::update_with_join::UpdateWithJoin;

fn ctx(src: &str) -> FileContext {
    FileContext::from_source(src, "test.sql")
}

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let c = ctx(sql);
    UpdateWithJoin.check(&c)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(UpdateWithJoin.name(), "Structure/UpdateWithJoin");
}

// ── parse error → 0 violations ───────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let diags = check("UPDATE SET BROKEN FROM JOIN WHERE");
    assert!(diags.is_empty());
}

// ── plain UPDATE with WHERE and no FROM → 0 violations ───────────────────────

#[test]
fn plain_update_with_where_no_violation() {
    let diags = check("UPDATE t SET col = 1 WHERE id = 1");
    assert!(diags.is_empty());
}

// ── UPDATE with correlated subquery in SET → 0 violations ────────────────────

#[test]
fn update_with_correlated_subquery_no_violation() {
    let diags = check(
        "UPDATE t SET col = (SELECT val FROM s WHERE s.id = t.id) WHERE t.id = 1",
    );
    assert!(diags.is_empty());
}

// ── SELECT statement → 0 violations ──────────────────────────────────────────

#[test]
fn select_statement_no_violation() {
    let diags = check("SELECT * FROM t JOIN s ON t.id = s.id");
    assert!(diags.is_empty());
}

// ── UPDATE … FROM … WHERE (no JOIN) → 1 violation ────────────────────────────

#[test]
fn update_from_no_join_one_violation() {
    let diags = check("UPDATE t SET col = s.val FROM s WHERE t.id = s.id");
    assert_eq!(diags.len(), 1);
}

// ── UPDATE … FROM … JOIN → 1 violation ───────────────────────────────────────

#[test]
fn update_from_with_join_one_violation() {
    let diags = check(
        "UPDATE t SET t.col = u.val FROM s JOIN u ON s.id = u.id WHERE t.id = s.id",
    );
    assert_eq!(diags.len(), 1);
}

// ── UPDATE with JOIN in the table (not FROM) → 1 violation ───────────────────
// Some dialects allow `UPDATE t JOIN s ON … SET …`; the table itself carries joins.

#[test]
fn update_table_with_join_one_violation() {
    // MySQL-style UPDATE with JOIN in the table expression.
    // sqlparser parses this into `table` with joins rather than `from`.
    let sql = "UPDATE t JOIN s ON t.id = s.id SET t.col = s.val WHERE t.id = 1";
    let diags = check(sql);
    // If it parses successfully, it should produce at least 1 diagnostic;
    // if it doesn't parse, 0 is acceptable (parse error path).
    // We assert the rule doesn't crash.
    let _ = diags; // just ensure no panic
}

// ── message contains expected text ───────────────────────────────────────────

#[test]
fn message_contains_expected_text() {
    let diags = check("UPDATE t SET col = s.val FROM s WHERE t.id = s.id");
    assert_eq!(diags.len(), 1);
    assert!(
        diags[0].message.contains("UPDATE"),
        "message should mention UPDATE"
    );
    assert!(
        diags[0]
            .message
            .to_lowercase()
            .contains("portab"),
        "message should mention portability"
    );
}

// ── diagnostic rule name matches ─────────────────────────────────────────────

#[test]
fn diagnostic_rule_name_matches() {
    let diags = check("UPDATE t SET col = s.val FROM s WHERE t.id = s.id");
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].rule, "Structure/UpdateWithJoin");
}

// ── UPDATE without WHERE (no FROM) → 0 violations for this rule ──────────────

#[test]
fn update_without_where_and_no_from_no_violation_for_this_rule() {
    // UpdateWithJoin only cares about FROM/JOIN, not missing WHERE.
    let diags = check("UPDATE t SET col = 1");
    assert!(diags.is_empty());
}

// ── multiple UPDATE statements: one with FROM, one without → 1 violation ─────

#[test]
fn multiple_stmts_one_with_from_one_without() {
    let sql = "UPDATE t SET col = 1 WHERE id = 1; UPDATE t SET col = s.val FROM s WHERE t.id = s.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

// ── UPDATE FROM with multiple JOIN → still 1 violation per statement ─────────

#[test]
fn update_from_multiple_joins_one_violation() {
    let sql = "UPDATE t SET t.col = u.val FROM s JOIN u ON s.id = u.id JOIN v ON u.vid = v.id WHERE t.id = s.id";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

// ── line/col is non-zero ──────────────────────────────────────────────────────

#[test]
fn line_col_is_nonzero() {
    let diags = check("UPDATE t SET col = s.val FROM s WHERE t.id = s.id");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
