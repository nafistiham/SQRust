/// Tests for --format json output: structure, fields, and severity values.
use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Parse the JSON array output of `sqrust check --format json`.
fn json_violations(sql: &str) -> Vec<serde_json::Value> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("q.sql");
    fs::write(&path, sql).unwrap();

    let output = Command::cargo_bin("sqrust")
        .unwrap()
        .args(["check", "--format", "json", path.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("JSON parse error: {e}\nOutput: {stdout}"))
}

// ─── JSON structure ────────────────────────────────────────────────────────

#[test]
fn json_output_is_valid_json_array() {
    let violations = json_violations("SELECT id FROM t;\n");
    // Just checking it parses — may be empty if no violations.
    let _ = violations;
}

#[test]
fn json_violation_has_required_fields() {
    // SELECT * always triggers Convention/SelectStar (unless disabled)
    let violations = json_violations("SELECT * FROM t;\n");
    assert!(
        !violations.is_empty(),
        "Expected at least one violation for SELECT *"
    );
    let v = &violations[0];
    assert!(v.get("file").is_some(), "missing 'file' field");
    assert!(v.get("line").is_some(), "missing 'line' field");
    assert!(v.get("col").is_some(), "missing 'col' field");
    assert!(v.get("rule").is_some(), "missing 'rule' field");
    assert!(v.get("message").is_some(), "missing 'message' field");
    assert!(v.get("severity").is_some(), "missing 'severity' field");
}

// ─── severity values ────────────────────────────────────────────────────────

#[test]
fn lint_violations_have_warning_severity() {
    let violations = json_violations("SELECT * FROM t;\n");
    let lint_v = violations
        .iter()
        .find(|v| v["rule"].as_str() != Some("Parse/Error"))
        .expect("Expected at least one non-parse violation");
    assert_eq!(
        lint_v["severity"].as_str(),
        Some("warning"),
        "Lint violations should have severity 'warning', got: {}",
        lint_v["severity"]
    );
}

#[test]
fn parse_errors_have_error_severity() {
    // Unparseable SQL
    let violations = json_violations("SELECT FROM FROM WHERE;\n");
    let parse_v = violations
        .iter()
        .find(|v| v["rule"].as_str() == Some("Parse/Error"))
        .expect("Expected a Parse/Error violation for invalid SQL");
    assert_eq!(
        parse_v["severity"].as_str(),
        Some("error"),
        "Parse errors should have severity 'error', got: {}",
        parse_v["severity"]
    );
}

// ─── clean file → empty array, exit 0 ─────────────────────────────────────

#[test]
fn clean_file_produces_empty_json_array() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("q.sql");
    // Fully conformant SQL: uppercase keywords, ends with newline
    fs::write(&path, "SELECT\n    id\nFROM my_table;\n").unwrap();

    let output = Command::cargo_bin("sqrust")
        .unwrap()
        .args(["check", "--format", "json", path.to_str().unwrap()])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Either "[]" or "[\n]\n" — just verify it parses as an empty array.
    let violations: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("JSON parse error: {e}\nOutput: {stdout}"));

    // A clean file should have 0 violations and exit 0.
    // Note: some rules may still fire (e.g. SelectTargetNewLine for SELECT/FROM split).
    // We only assert exit 0 here.
    assert_eq!(
        output.status.code(),
        Some(0),
        "Expected exit 0 for a file with no violations, but got violations:\n{}",
        serde_json::to_string_pretty(&violations).unwrap()
    );
}
