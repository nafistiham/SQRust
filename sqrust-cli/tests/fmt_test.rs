//! End-to-end tests for the `sqrust fmt` subcommand.
//!
//! `fmt` rewrites the user's source files in place, so every defect here is
//! data loss. These tests invoke the real binary, read the file back, and
//! assert on its exact bytes — the only way to catch corruption that
//! rule-level unit tests miss.

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Runs `sqrust fmt` on `path` and returns the process exit code.
fn run_fmt(path: &std::path::Path) -> Option<i32> {
    Command::cargo_bin("sqrust")
        .unwrap()
        .args(["fmt", path.to_str().unwrap()])
        .output()
        .unwrap()
        .status
        .code()
}

/// Writes `content` to a temp .sql file, runs `fmt`, returns (exit_code, new_content).
fn fmt_roundtrip(content: &str) -> (Option<i32>, String) {
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, content).unwrap();
    let code = run_fmt(&sql);
    let after = fs::read_to_string(&sql).unwrap();
    (code, after)
}

// ─── Must not crash ────────────────────────────────────────────────────────

/// Regression: the NotEqual fix() skip-range filter selected ranges that had
/// already ended, producing a reversed slice and a panic. Two string literals
/// in one file were enough — i.e. essentially every real SQL file.
#[test]
fn fmt_does_not_panic_on_multiple_string_literals() {
    let (code, _) = fmt_roundtrip("SELECT * FROM t WHERE a = 'x' AND b = 'y';\n");
    assert_ne!(code, Some(101), "fmt panicked on SQL with two string literals");
    assert_eq!(code, Some(0), "expected clean exit");
}

#[test]
fn fmt_does_not_panic_on_string_literal_and_comment() {
    let (code, _) = fmt_roundtrip("-- a note\nSELECT 'x' FROM t;\n");
    assert_ne!(code, Some(101), "fmt panicked on literal + comment");
}

#[test]
fn fmt_does_not_panic_on_two_comments() {
    let (code, _) = fmt_roundtrip("-- one\n-- two\nSELECT 1;\n");
    assert_ne!(code, Some(101), "fmt panicked on two comments");
}

// ─── Must not corrupt bytes ────────────────────────────────────────────────

/// Regression: a fix() rebuilt the file with `b as char`, which splits
/// multi-byte UTF-8 sequences (café -> cafÃ©). The file had no violations at
/// all, yet was rewritten.
#[test]
fn fmt_preserves_multibyte_utf8_in_clean_file() {
    let src = "SELECT 'café';\n";
    let (_, after) = fmt_roundtrip(src);
    assert_eq!(after, src, "fmt corrupted a clean non-ASCII file");
}

#[test]
fn fmt_preserves_multibyte_utf8_while_fixing() {
    // Has a real violation (space before semicolon) AND non-ASCII content.
    let (_, after) = fmt_roundtrip("SELECT 'café' ;\n");
    assert_eq!(after, "SELECT 'café';\n", "fmt mangled non-ASCII while fixing");
}

#[test]
fn fmt_preserves_wide_multibyte_characters() {
    let src = "SELECT '中文テスト' AS label;\n";
    let (_, after) = fmt_roundtrip(src);
    assert_eq!(after, src, "fmt corrupted multi-byte CJK content");
}

/// Regression: fixes built on `str::lines()` + `join("\n")` silently
/// normalised CRLF to LF, rewriting files that had nothing to fix.
#[test]
fn fmt_preserves_crlf_line_endings_in_clean_file() {
    let src = "SELECT 1\r\nFROM t\r\n";
    let (_, after) = fmt_roundtrip(src);
    assert_eq!(after, src, "fmt stripped CR from a clean CRLF file");
}

#[test]
fn fmt_leaves_clean_file_byte_identical() {
    let src = "SELECT id\nFROM users\nWHERE id > 0;\n";
    let (_, after) = fmt_roundtrip(src);
    assert_eq!(after, src, "fmt modified a file with nothing to fix");
}

// ─── Must actually fix ─────────────────────────────────────────────────────

#[test]
fn fmt_removes_trailing_whitespace() {
    let (_, after) = fmt_roundtrip("SELECT 1   \nFROM t\n");
    assert_eq!(after, "SELECT 1\nFROM t\n");
}

#[test]
fn fmt_removes_whitespace_before_semicolon() {
    let (_, after) = fmt_roundtrip("SELECT 1 ;\n");
    assert_eq!(after, "SELECT 1;\n");
}

#[test]
fn fmt_does_not_rewrite_inside_string_literals() {
    // The double space and the `!=` are inside a literal — both must survive.
    let src = "SELECT 'a  b != c' AS s;\n";
    let (_, after) = fmt_roundtrip(src);
    assert_eq!(after, src, "fmt rewrote content inside a string literal");
}

// ─── Must be idempotent and produce parseable SQL ──────────────────────────

#[test]
fn fmt_is_idempotent() {
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, "SELECT a  ,  b   \nFROM t\nWHERE x = 1 ;\n").unwrap();

    run_fmt(&sql);
    let first = fs::read_to_string(&sql).unwrap();
    run_fmt(&sql);
    let second = fs::read_to_string(&sql).unwrap();

    assert_eq!(first, second, "fmt is not idempotent — second run changed the file");
}

// ─── Unparseable input (dbt Jinja) ─────────────────────────────────────────

/// dbt models contain Jinja that sqlparser-rs cannot parse. Text-level fixes
/// are still applied — that is what makes `fmt` useful for dbt — but the
/// parse-check safety net cannot run, so the user must be told.
#[test]
fn fmt_warns_when_formatting_an_unparseable_file() {
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, "{{ config(materialized='table') }}\nSELECT *   \nFROM {{ ref('stg') }} ;\n")
        .unwrap();

    let output = Command::cargo_bin("sqrust")
        .unwrap()
        .args(["fmt", sql.to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("could not be parsed"),
        "expected an unparseable-file warning, got stderr:\n{stderr}"
    );

    // The text-level fixes must still have been applied, and the Jinja kept.
    let after = fs::read_to_string(&sql).unwrap();
    assert_eq!(
        after,
        "{{ config(materialized='table') }}\nSELECT *\nFROM {{ ref('stg') }};\n"
    );
}

#[test]
fn fmt_does_not_warn_on_parseable_file() {
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, "SELECT 1 ;\n").unwrap();

    let output = Command::cargo_bin("sqrust")
        .unwrap()
        .args(["fmt", sql.to_str().unwrap()])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("could not be parsed"),
        "unexpected warning on a file that parses fine:\n{stderr}"
    );
}

/// The formatted output must still be valid SQL. Without this, a bad fix can
/// silently write syntactically broken SQL over the user's source.
#[test]
fn fmt_output_still_parses() {
    let dir = TempDir::new().unwrap();
    let sql = dir.path().join("q.sql");
    fs::write(&sql, "SELECT a  ,  b FROM t WHERE x != 1 AND y = 'z' ;\n").unwrap();

    run_fmt(&sql);

    let output = Command::cargo_bin("sqrust")
        .unwrap()
        .args(["check", sql.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("Parse/Error"),
        "fmt produced SQL that no longer parses:\n{}\n--- output ---\n{stdout}",
        fs::read_to_string(&sql).unwrap()
    );
}
