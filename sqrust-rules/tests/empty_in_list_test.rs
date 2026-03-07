use sqrust_core::FileContext;
use sqrust_core::Rule;
use sqrust_rules::lint::empty_in_list::EmptyInList;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    let ctx = FileContext::from_source(sql, "test.sql");
    EmptyInList.check(&ctx)
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(EmptyInList.name(), "Lint/EmptyInList");
}

#[test]
fn normal_in_list_no_violation() {
    let sql = "SELECT * FROM t WHERE id IN (1, 2, 3)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn empty_in_list_one_violation() {
    let sql = "SELECT * FROM t WHERE id IN ()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn not_in_empty_flagged() {
    let sql = "SELECT * FROM t WHERE id NOT IN ()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn empty_in_with_spaces_flagged() {
    let sql = "SELECT * FROM t WHERE id IN (   )";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn empty_in_in_string_not_flagged() {
    let sql = "SELECT * FROM t WHERE note = 'IN ()'";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn empty_in_in_comment_not_flagged() {
    let sql = "-- WHERE id IN ()\nSELECT 1";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn two_empty_in_lists_two_violations() {
    let sql = "SELECT * FROM t WHERE id IN () AND cat IN ()";
    let diags = check(sql);
    assert_eq!(diags.len(), 2);
}

#[test]
fn empty_in_in_having_flagged() {
    let sql = "SELECT dept FROM t GROUP BY dept HAVING dept IN ()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}

#[test]
fn nonempty_in_no_violation() {
    let sql = "SELECT * FROM t WHERE x IN (NULL)";
    let diags = check(sql);
    assert!(diags.is_empty());
}

#[test]
fn message_content() {
    let sql = "SELECT * FROM t WHERE id IN ()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    let msg = diags[0].message.to_lowercase();
    assert!(
        msg.contains("empty") || msg.contains("false"),
        "message should mention 'empty' or 'FALSE': {}",
        diags[0].message
    );
}

#[test]
fn line_nonzero() {
    let sql = "SELECT * FROM t WHERE id IN ()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].line >= 1);
}

#[test]
fn col_nonzero() {
    let sql = "SELECT * FROM t WHERE id IN ()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].col >= 1);
}

#[test]
fn lowercase_in_flagged() {
    let sql = "select * from t where id in ()";
    let diags = check(sql);
    assert_eq!(diags.len(), 1);
}
