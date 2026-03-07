use sqrust_core::FileContext;
use sqrust_rules::layout::max_statement_length::MaxStatementLength;
use sqrust_core::Rule;

fn check_default(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    MaxStatementLength::default().check(&ctx)
}

fn check_with_max(sql: &str, max_lines: usize) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    MaxStatementLength { max_lines }.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    let sql = (0..51).map(|i| format!("-- line {}\n", i)).collect::<String>() + ";";
    let diags = check_default(&sql);
    assert_eq!(diags[0].rule, "Layout/MaxStatementLength");
}

#[test]
fn short_statement_no_violation() {
    // 5-line statement, default max 50
    let sql = "SELECT a,\n  b,\n  c\nFROM t\nWHERE x = 1;";
    let diags = check_default(sql);
    assert!(diags.is_empty());
}

#[test]
fn exactly_50_lines_no_violation() {
    // Build a statement with exactly 50 lines (lines 1..=50, terminated by ;)
    let mut lines: Vec<String> = (0..49).map(|i| format!("-- line {}", i)).collect();
    lines.push(";".to_string());
    let sql = lines.join("\n");
    let diags = check_default(&sql);
    assert!(diags.is_empty());
}

#[test]
fn fifty_one_lines_one_violation() {
    // 51-line statement — over the limit
    let mut lines: Vec<String> = (0..50).map(|i| format!("-- line {}", i)).collect();
    lines.push(";".to_string());
    let sql = lines.join("\n");
    let diags = check_default(&sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn two_short_statements_no_violation() {
    let sql = "SELECT 1;\nSELECT 2;";
    let diags = check_default(sql);
    assert!(diags.is_empty());
}

#[test]
fn one_long_one_short_one_violation() {
    // First statement: 51 lines, second: 1 line
    let mut lines: Vec<String> = (0..50).map(|i| format!("-- line {}", i)).collect();
    lines.push(";".to_string());
    lines.push("SELECT 1;".to_string());
    let sql = lines.join("\n");
    let diags = check_default(&sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn custom_max_5_six_line_violation() {
    // 6 lines with max_lines = 5
    let sql = "SELECT a,\n  b,\n  c,\n  d,\n  e,\n  f;";
    let diags = check_with_max(sql, 5);
    assert_eq!(diags.len(), 1);
}

#[test]
fn custom_max_5_five_line_no_violation() {
    // 5 lines with max_lines = 5 — exactly at limit
    let sql = "SELECT a,\n  b,\n  c,\n  d,\n  e;";
    let diags = check_with_max(sql, 5);
    assert!(diags.is_empty());
}

#[test]
fn default_max_is_50() {
    let rule = MaxStatementLength::default();
    assert_eq!(rule.max_lines, 50);
}

#[test]
fn message_contains_line_count_and_max() {
    // 51-line statement with default max 50
    let mut lines: Vec<String> = (0..50).map(|i| format!("-- line {}", i)).collect();
    lines.push(";".to_string());
    let sql = lines.join("\n");
    let diags = check_default(&sql);
    let msg = &diags[0].message;
    // Message must contain the actual line count (51) and the max (50)
    assert!(
        msg.contains("51") && msg.contains("50"),
        "Unexpected message: {}",
        msg
    );
}

#[test]
fn line_col_nonzero() {
    // The diagnostic should point to a line and col >= 1
    let mut lines: Vec<String> = (0..50).map(|i| format!("-- line {}", i)).collect();
    lines.push(";".to_string());
    let sql = lines.join("\n");
    let diags = check_default(&sql);
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn statement_without_semicolon_no_violation() {
    // No semicolons — the whole file is one statement; 3 lines is fine with default max 50
    let sql = "SELECT a\nFROM t\nWHERE x = 1";
    let diags = check_default(sql);
    assert!(diags.is_empty());
}

#[test]
fn empty_statement_no_violation() {
    // Blank lines between semicolons produce effectively empty statements
    let sql = "SELECT 1;\n\n;\n\nSELECT 2;";
    let diags = check_default(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_long_statements_two_violations() {
    // Both statements exceed the limit with max_lines = 3
    let sql = "SELECT a,\n  b,\n  c,\n  d;\nSELECT e,\n  f,\n  g,\n  h;";
    let diags = check_with_max(sql, 3);
    assert_eq!(diags.len(), 2);
}

#[test]
fn violation_points_to_first_line_of_statement() {
    // The diagnostic line should be the start of the long statement
    let mut lines: Vec<String> = (0..50).map(|i| format!("-- line {}", i)).collect();
    lines.push(";".to_string());
    let sql = lines.join("\n");
    let diags = check_default(&sql);
    // Statement starts at line 1
    assert_eq!(diags[0].line, 1);
}
