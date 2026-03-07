use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::convention::no_current_timestamp_in_where::NoCurrentTimestampInWhere;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoCurrentTimestampInWhere.check(&ctx)
}

// ── rule name ─────────────────────────────────────────────────────────────────

#[test]
fn rule_name_is_correct() {
    assert_eq!(
        NoCurrentTimestampInWhere.name(),
        "Convention/NoCurrentTimestampInWhere"
    );
}

// ── parse error ───────────────────────────────────────────────────────────────

#[test]
fn parse_error_returns_no_violations() {
    let ctx = FileContext::from_source("SELECTT INVALID GARBAGE @@##", "test.sql");
    if !ctx.parse_errors.is_empty() {
        let diags = NoCurrentTimestampInWhere.check(&ctx);
        assert!(diags.is_empty());
    }
}

// ── WHERE violations ──────────────────────────────────────────────────────────

#[test]
fn current_timestamp_in_where_violation() {
    let diags = check(
        "SELECT id FROM t WHERE created_at > CURRENT_TIMESTAMP",
    );
    assert_eq!(diags.len(), 1);
}

#[test]
fn now_function_in_where_violation() {
    let diags = check("SELECT id FROM t WHERE created_at > NOW()");
    assert_eq!(diags.len(), 1);
}

#[test]
fn getdate_in_where_violation() {
    let diags = check("SELECT id FROM t WHERE created_at < GETDATE()");
    assert_eq!(diags.len(), 1);
}

#[test]
fn sysdate_in_where_violation() {
    let diags = check("SELECT id FROM t WHERE created_at < SYSDATE()");
    assert_eq!(diags.len(), 1);
}

// ── no violation cases ────────────────────────────────────────────────────────

#[test]
fn current_timestamp_in_select_no_violation() {
    // CURRENT_TIMESTAMP in the SELECT list is fine — deterministic per-query
    let diags = check("SELECT CURRENT_TIMESTAMP AS now FROM t");
    assert!(diags.is_empty());
}

#[test]
fn no_timestamp_function_no_violation() {
    let diags = check("SELECT id FROM t WHERE id = 1 AND name = 'foo'");
    assert!(diags.is_empty());
}

// ── HAVING violation ──────────────────────────────────────────────────────────

#[test]
fn current_timestamp_in_having_violation() {
    let diags = check(
        "SELECT dept_id, MAX(created_at) FROM t GROUP BY dept_id HAVING MAX(created_at) > CURRENT_TIMESTAMP",
    );
    assert_eq!(diags.len(), 1);
}

// ── JOIN ON violation ─────────────────────────────────────────────────────────

#[test]
fn now_in_join_on_violation() {
    let diags = check(
        "SELECT t.id FROM t JOIN u ON t.created_at > NOW() AND t.id = u.id",
    );
    assert_eq!(diags.len(), 1);
}

// ── message content ───────────────────────────────────────────────────────────

#[test]
fn message_contains_useful_text() {
    let diags = check("SELECT id FROM t WHERE created_at > CURRENT_TIMESTAMP");
    assert_eq!(diags.len(), 1);
    assert!(!diags[0].message.is_empty());
    // Message should mention non-deterministic or the function name
    let msg_upper = diags[0].message.to_uppercase();
    let has_useful = msg_upper.contains("CURRENT_TIMESTAMP")
        || msg_upper.contains("NOW")
        || msg_upper.contains("NON-DETERMINISTIC")
        || msg_upper.contains("NONDETERMINISTIC")
        || msg_upper.contains("WHERE");
    assert!(has_useful, "message not useful enough: {}", diags[0].message);
}

// ── position ──────────────────────────────────────────────────────────────────

#[test]
fn line_col_nonzero() {
    let diags = check("SELECT id FROM t WHERE created_at > CURRENT_TIMESTAMP");
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

// ── CURRENT_DATE — flagged for consistency ────────────────────────────────────

#[test]
fn current_date_in_where_no_violation() {
    // CURRENT_DATE is deterministic per-query execution (stable within one query).
    // We choose NOT to flag it.
    let diags = check("SELECT id FROM t WHERE created_date = CURRENT_DATE");
    assert!(diags.is_empty());
}

// ── case-insensitive ──────────────────────────────────────────────────────────

#[test]
fn lowercase_now_in_where_violation() {
    let diags = check("SELECT id FROM t WHERE created_at > now()");
    assert_eq!(diags.len(), 1);
}
