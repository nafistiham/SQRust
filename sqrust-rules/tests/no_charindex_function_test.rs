use sqrust_core::FileContext;
use sqrust_rules::convention::no_charindex_function::NoCharindexFunction;
use sqrust_core::Rule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    NoCharindexFunction.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NoCharindexFunction.name(), "Convention/NoCharindexFunction");
}

// CHARINDEX tests

#[test]
fn charindex_basic_violation() {
    let diags = check("SELECT CHARINDEX('a', col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn charindex_lowercase_violation() {
    let diags = check("SELECT charindex('a', col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn charindex_mixed_case_violation() {
    let diags = check("SELECT CharIndex('a', col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn charindex_message_is_correct() {
    let diags = check("SELECT CHARINDEX('a', col) FROM t");
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].message,
        "CHARINDEX() is SQL Server-specific; use POSITION(substring IN string) for standard SQL"
    );
}

// LOCATE tests

#[test]
fn locate_basic_violation() {
    let diags = check("SELECT LOCATE('a', col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn locate_lowercase_violation() {
    let diags = check("SELECT locate('a', col) FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn locate_message_is_correct() {
    let diags = check("SELECT LOCATE('a', col) FROM t");
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].message,
        "LOCATE() is MySQL-specific; use POSITION(substring IN string) for standard SQL"
    );
}

// INSTR tests

#[test]
fn instr_basic_violation() {
    let diags = check("SELECT INSTR(col, 'a') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn instr_lowercase_violation() {
    let diags = check("SELECT instr(col, 'a') FROM t");
    assert_eq!(diags.len(), 1);
}

#[test]
fn instr_message_is_correct() {
    let diags = check("SELECT INSTR(col, 'a') FROM t");
    assert!(!diags.is_empty());
    assert_eq!(
        diags[0].message,
        "INSTR() is Oracle/MySQL-specific; use POSITION(substring IN string) for standard SQL"
    );
}

// No-violation tests

#[test]
fn position_no_violation() {
    let diags = check("SELECT POSITION('a' IN col) FROM t");
    assert!(diags.is_empty());
}

#[test]
fn charindex_in_string_no_violation() {
    let diags = check("SELECT 'CHARINDEX(x, y)' FROM t");
    assert!(diags.is_empty());
}

#[test]
fn locate_in_comment_no_violation() {
    let diags = check("-- LOCATE('a', col)\nSELECT col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn instr_in_string_no_violation() {
    let diags = check("SELECT 'INSTR(col, a)' FROM t");
    assert!(diags.is_empty());
}

// Word-boundary tests: column names that contain the function name as prefix/suffix should NOT flag

#[test]
fn charindex_as_column_prefix_no_violation() {
    // "charindex_col" - word char before the next identifier start
    let diags = check("SELECT charindex_col FROM t");
    assert!(diags.is_empty());
}

#[test]
fn instr_as_column_suffix_no_violation() {
    // "my_instr" - word char immediately before "instr"
    let diags = check("SELECT my_instr FROM t");
    assert!(diags.is_empty());
}

// Multiple violations

#[test]
fn multiple_violations_in_one_query() {
    let diags = check("SELECT CHARINDEX('a', col), LOCATE('b', col), INSTR(col, 'c') FROM t");
    assert_eq!(diags.len(), 3);
}

// Line/col reporting

#[test]
fn line_col_is_nonzero() {
    let diags = check("SELECT CHARINDEX('x', col) FROM t");
    assert!(!diags.is_empty());
    assert!(diags[0].line >= 1);
    assert!(diags[0].col >= 1);
}
