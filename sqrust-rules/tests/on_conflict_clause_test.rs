use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::on_conflict_clause::OnConflictClause;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    OnConflictClause.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(OnConflictClause.name(), "Lint/OnConflictClause");
}

#[test]
fn on_conflict_one_violation() {
    let sql = "INSERT INTO t (a) VALUES (1) ON CONFLICT (a) DO NOTHING";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn on_conflict_case_insensitive_upper() {
    let sql = "INSERT INTO t (a) VALUES (1) ON CONFLICT (a) DO NOTHING";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn on_conflict_case_insensitive_lower() {
    let sql = "insert into t (a) values (1) on conflict (a) do nothing";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn on_conflict_mixed_case() {
    let sql = "INSERT INTO t (a) VALUES (1) On Conflict (a) Do Nothing";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn plain_insert_no_violation() {
    let sql = "INSERT INTO t (a) VALUES (1)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn insert_into_select_no_violation() {
    let sql = "INSERT INTO t SELECT a FROM src";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn select_no_violation() {
    let sql = "SELECT * FROM t WHERE a = 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_mentions_postgresql_or_sqlite() {
    let sql = "INSERT INTO t (a) VALUES (1) ON CONFLICT (a) DO NOTHING";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("postgresql") || msg.contains("sqlite"),
        "message should mention PostgreSQL or SQLite: {}",
        diags[0].message
    );
}

#[test]
fn message_suggests_merge_or_alternative() {
    let sql = "INSERT INTO t (a) VALUES (1) ON CONFLICT (a) DO NOTHING";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("merge") || msg.contains("on duplicate key"),
        "message should suggest MERGE or alternative: {}",
        diags[0].message
    );
}

#[test]
fn two_on_conflicts_two_violations() {
    let sql = concat!(
        "INSERT INTO t1 (a) VALUES (1) ON CONFLICT (a) DO NOTHING;\n",
        "INSERT INTO t2 (b) VALUES (2) ON CONFLICT (b) DO UPDATE SET b = excluded.b",
    );
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn line_col_nonzero() {
    let sql = "INSERT INTO t (a) VALUES (1) ON CONFLICT (a) DO NOTHING";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn on_conflict_on_second_line() {
    let sql = "INSERT INTO t (a) VALUES (1)\nON CONFLICT (a) DO NOTHING";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].line, 2);
}

#[test]
fn on_conflict_in_string_literal_no_violation() {
    let sql = "SELECT 'ON CONFLICT (a) DO NOTHING' AS q";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn on_conflict_in_comment_no_violation() {
    let sql = "-- ON CONFLICT (a) DO NOTHING\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn parse_error_still_detects_violation() {
    // Source-level scan works regardless of parse errors
    let sql = "INSERT INTO t (a) VALUES (### bad ON CONFLICT (a) DO NOTHING";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn on_conflict_do_update_violation() {
    let sql = "INSERT INTO t (a, b) VALUES (1, 2) ON CONFLICT (a) DO UPDATE SET b = excluded.b";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
