use sqrust_core::FileContext;
use sqrust_rules::layout::long_lines::LongLines;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    LongLines::default().check(&ctx)
}

fn check_with_max(sql: &str, max_length: usize) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    LongLines { max_length }.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LongLines::default().name(), "Layout/LongLines");
}

#[test]
fn short_file_has_no_violations() {
    let diags = check("SELECT id\nFROM users\n");
    assert!(diags.is_empty());
}

#[test]
fn empty_file_has_no_violations() {
    let diags = check("");
    assert!(diags.is_empty());
}

#[test]
fn line_of_exactly_120_chars_has_no_violation() {
    // 120 'a' characters — at the limit, not over
    let line = "a".repeat(120);
    let diags = check(&line);
    assert!(diags.is_empty());
}

#[test]
fn line_of_121_chars_has_one_violation() {
    // 121 'a' characters — one over the limit
    let line = "a".repeat(121);
    let diags = check(&line);
    assert_eq!(diags.len(), 1);
}

#[test]
fn line_of_121_chars_violation_has_correct_line_and_col() {
    let line = "a".repeat(121);
    let diags = check(&line);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[0].col, 121); // max_length + 1 = 120 + 1 = 121
}

#[test]
fn line_of_150_chars_has_correct_col_and_message() {
    let line = "a".repeat(150);
    let diags = check(&line);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 121); // max_length + 1 = 120 + 1 = 121
    assert_eq!(
        diags[0].message,
        "Line is 150 characters, maximum is 120"
    );
}

#[test]
fn multiple_long_lines_all_flagged() {
    let long_line = "a".repeat(130);
    let sql = format!("{}\n{}\nSELECT 1\n{}\n", long_line, long_line, long_line);
    let diags = check(&sql);
    assert_eq!(diags.len(), 3);
    assert_eq!(diags[0].line, 1);
    assert_eq!(diags[1].line, 2);
    assert_eq!(diags[2].line, 4);
}

#[test]
fn custom_max_length_80_flags_line_over_80() {
    let line_81 = "a".repeat(81);
    let diags = check_with_max(&line_81, 80);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].col, 81); // max_length + 1 = 80 + 1 = 81
    assert_eq!(
        diags[0].message,
        "Line is 81 characters, maximum is 80"
    );
}

#[test]
fn custom_max_length_80_does_not_flag_line_of_exactly_80() {
    let line_80 = "a".repeat(80);
    let diags = check_with_max(&line_80, 80);
    assert!(diags.is_empty());
}

#[test]
fn unicode_chars_counted_by_char_not_byte() {
    // Each '€' is 3 bytes but 1 char.
    // 121 '€' chars should be 1 violation; 120 should be 0.
    let line_120 = "€".repeat(120);
    let diags = check(&line_120);
    assert!(diags.is_empty(), "120 unicode chars should not violate");

    let line_121 = "€".repeat(121);
    let diags = check(&line_121);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].message, "Line is 121 characters, maximum is 120");
}

#[test]
fn violation_carries_correct_rule_name() {
    let line = "a".repeat(121);
    let diags = check(&line);
    assert_eq!(diags[0].rule, "Layout/LongLines");
}
