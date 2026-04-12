use assert_cmd::Command;
use predicates::prelude::*;
use predicates::str::contains;
use std::fs;
use tempfile::TempDir;

/// BigQuery SQL: backtick identifiers are valid in BigQuery, not in strict ANSI.
const BIGQUERY_BACKTICK_SQL: &str =
    "SELECT `id`, `name` FROM `my_project.my_dataset.my_table`;";

/// BigQuery SQL: QUALIFY clause is BigQuery/Snowflake-specific — not in ANSI SQL.
const BIGQUERY_QUALIFY_SQL: &str =
    "SELECT id, ROW_NUMBER() OVER (PARTITION BY grp ORDER BY id) AS rn FROM t QUALIFY rn = 1;";

/// Plain ANSI SQL that any dialect should parse without errors.
const ANSI_SQL: &str = "SELECT id FROM my_table;";

// ─── --dialect bigquery ────────────────────────────────────────────────────

#[test]
fn dialect_bigquery_flag_parses_backtick_identifiers_without_parse_error() {
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, BIGQUERY_BACKTICK_SQL).unwrap();

    // May exit 0 (no violations) or 1 (lint violations), but must NOT have
    // a Parse/Error diagnostic in its output.
    let output = Command::cargo_bin("sqrust")
        .unwrap()
        .args(["check", "--dialect", "bigquery", sql.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Parse/Error"),
        "Got unexpected parse error in bigquery mode:\n{stdout}"
    );
    // Exit code must be 0 or 1, not 2 (which signals a tool-level error).
    assert_ne!(
        output.status.code(),
        Some(2),
        "Expected exit 0 or 1, got 2"
    );
}

// ─── BigQuery dialect accepts QUALIFY; ANSI dialect rejects it ─────────────

#[test]
fn dialect_bigquery_flag_accepts_qualify_clause() {
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, BIGQUERY_QUALIFY_SQL).unwrap();

    let output = Command::cargo_bin("sqrust")
        .unwrap()
        .args(["check", "--dialect", "bigquery", sql.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Parse/Error"),
        "Got unexpected parse error for QUALIFY in bigquery mode:\n{stdout}"
    );
}


// ─── unknown dialect exits 2 ──────────────────────────────────────────────

#[test]
fn dialect_unknown_value_exits_2_with_clear_message() {
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, ANSI_SQL).unwrap();

    Command::cargo_bin("sqrust")
        .unwrap()
        .args(["check", "--dialect", "oracle_99", sql.to_str().unwrap()])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("unknown dialect").and(contains("oracle_99")));
}

// ─── --dialect flag overrides sqrust.toml ─────────────────────────────────

#[test]
fn dialect_flag_overrides_toml_config() {
    // Config says ansi, flag says bigquery — flag must win.
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, BIGQUERY_BACKTICK_SQL).unwrap();
    fs::write(
        dir.path().join("sqrust.toml"),
        "[sqrust]\ndialect = \"ansi\"\n",
    )
    .unwrap();

    let output = Command::cargo_bin("sqrust")
        .unwrap()
        .args(["check", "--dialect", "bigquery", sql.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Parse/Error"),
        "--dialect flag should override config, but got parse error:\n{stdout}"
    );
}

// ─── sqrust.toml dialect validated ────────────────────────────────────────

#[test]
fn toml_unknown_dialect_exits_2_with_clear_message() {
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, ANSI_SQL).unwrap();
    fs::write(
        dir.path().join("sqrust.toml"),
        "[sqrust]\ndialect = \"oracle_v19\"\n",
    )
    .unwrap();

    Command::cargo_bin("sqrust")
        .unwrap()
        .args(["check", sql.to_str().unwrap()])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("unknown dialect").and(contains("oracle_v19")));
}

// ─── fmt --dialect flag ────────────────────────────────────────────────────

#[test]
fn fmt_accepts_dialect_flag() {
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, ANSI_SQL).unwrap();

    // Must not exit 2 (tool error) when a valid dialect flag is passed to fmt.
    let output = Command::cargo_bin("sqrust")
        .unwrap()
        .args(["fmt", "--dialect", "ansi", sql.to_str().unwrap()])
        .output()
        .unwrap();

    assert_ne!(
        output.status.code(),
        Some(2),
        "fmt --dialect ansi should not exit 2, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn fmt_unknown_dialect_exits_2() {
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, ANSI_SQL).unwrap();

    Command::cargo_bin("sqrust")
        .unwrap()
        .args(["fmt", "--dialect", "badval", sql.to_str().unwrap()])
        .assert()
        .failure()
        .code(2)
        .stderr(contains("unknown dialect").and(contains("badval")));
}
