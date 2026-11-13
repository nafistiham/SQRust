use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn missing_targets_fail_check_and_fmt() {
    let dir = TempDir::new().unwrap();
    for action in ["check", "fmt"] {
        Command::cargo_bin("sqrust")
            .unwrap()
            .current_dir(dir.path())
            .args([action, "missing.sql"])
            .assert()
            .failure();
    }
}

#[test]
fn directories_with_sql_extension_are_not_read_as_files() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("archive.sql")).unwrap();
    Command::cargo_bin("sqrust")
        .unwrap()
        .current_dir(dir.path())
        .args(["check", "."])
        .assert()
        .success();
}

#[test]
fn empty_json_check_returns_an_array() {
    let dir = TempDir::new().unwrap();
    let result = Command::cargo_bin("sqrust")
        .unwrap()
        .current_dir(dir.path())
        .args(["check", "--format", "json", "."])
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
        serde_json::json!([])
    );
}

#[test]
fn overlapping_path_spellings_do_not_duplicate_diagnostics() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("q.sql"), "SELEC 1\n").unwrap();
    let result = Command::cargo_bin("sqrust")
        .unwrap()
        .current_dir(dir.path())
        .args(["check", "--format", "json", "q.sql", "./q.sql", "."])
        .output()
        .unwrap();
    let diagnostics: Vec<serde_json::Value> = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        diagnostics
            .iter()
            .filter(|d| d["rule"] == "Parse/Error")
            .count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn symlink_aliases_are_linted_only_once() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("q.sql"), "SELEC 1\n").unwrap();
    std::os::unix::fs::symlink("q.sql", dir.path().join("alias.sql")).unwrap();
    let result = Command::cargo_bin("sqrust")
        .unwrap()
        .current_dir(dir.path())
        .args(["check", "--format", "json", "."])
        .output()
        .unwrap();
    let diagnostics: Vec<serde_json::Value> = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        diagnostics
            .iter()
            .filter(|d| d["rule"] == "Parse/Error")
            .count(),
        1
    );
}
