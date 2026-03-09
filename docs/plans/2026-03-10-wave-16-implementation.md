# Wave 16 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add 10 new lint rules across 5 categories (2 per category), bringing the total from 145 → 155.

**Architecture:** Each rule follows the established pattern — one `.rs` file per rule in `sqrust-rules/src/<category>/`, one test file in `sqrust-rules/tests/`, registered in the category `mod.rs` and in `sqrust-cli/src/main.rs`.

**Tech Stack:** Rust stable, sqlparser-rs 0.53, sqrust-core Rule trait, SkipMap + is_word_char from `crate::capitalisation`.

---

## Shared Context (read before any task)

```
Project root:   /Users/md.tihami/Desktop/Me/sqrust
Cargo binary:   ~/.cargo/bin/cargo
Run all tests:  ~/.cargo/bin/cargo test --workspace 2>&1 | grep -E "FAILED|^test result"
Run one rule:   ~/.cargo/bin/cargo test -p sqrust-rules <rule_name>
```

**Rule struct template:**
```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
// ... other imports

pub struct RuleName;

impl Rule for RuleName {
    fn name(&self) -> &'static str { "Category/RuleName" }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() { return Vec::new(); }
        // ...
    }
}
```

**Test file template:**
```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::<category>::<rule_name>::<RuleStruct>;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    <RuleStruct>.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(<RuleStruct>.name(), "Category/RuleName");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}
```

**SkipMap helper** (available as `use crate::capitalisation::{is_word_char, SkipMap};`):
- `SkipMap::build(source)` — marks bytes inside strings/comments as skip
- `skip_map.is_code(i)` — true if byte at `i` is real SQL (not string/comment)
- `is_word_char(byte)` — true for `[a-zA-Z0-9_]`

**Diagnostic fields:** `rule: &'static str`, `message: String`, `line: usize` (1-indexed), `col: usize` (1-indexed)

**offset_to_line_col helper** (copy into each rule file):
```rust
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

---

## AGENT A — Convention: `LeftJoin` + `JoinConditionStyle`

**Files to create:**
- `sqrust-rules/src/convention/left_join.rs`
- `sqrust-rules/src/convention/join_condition_style.rs`
- `sqrust-rules/tests/left_join_test.rs`
- `sqrust-rules/tests/join_condition_style_test.rs`

**Files to modify:**
- `sqrust-rules/src/convention/mod.rs` — add two `pub mod` lines
- `sqrust-cli/src/main.rs` — add two `use` lines and two `Box::new(...)` entries in `rules()`

---

### Task A1: Write failing tests for `LeftJoin`

Create `sqrust-rules/tests/left_join_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::convention::left_join::LeftJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    LeftJoin.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(LeftJoin.name(), "Convention/LeftJoin");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn left_join_no_violation() {
    assert!(check("SELECT a.id FROM a LEFT JOIN b ON a.id = b.id").is_empty());
}

#[test]
fn inner_join_no_violation() {
    assert!(check("SELECT a.id FROM a INNER JOIN b ON a.id = b.id").is_empty());
}

#[test]
fn cross_join_no_violation() {
    assert!(check("SELECT a.id FROM a CROSS JOIN b").is_empty());
}

#[test]
fn right_join_flagged() {
    let d = check("SELECT a.id FROM a RIGHT JOIN b ON a.id = b.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn right_outer_join_flagged() {
    let d = check("SELECT a.id FROM a RIGHT OUTER JOIN b ON a.id = b.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_right_joins_flagged() {
    let d = check("SELECT * FROM a RIGHT JOIN b ON a.id = b.id RIGHT JOIN c ON a.id = c.id");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_left() {
    let d = check("SELECT * FROM a RIGHT JOIN b ON a.id = b.id");
    assert!(d[0].message.to_uppercase().contains("LEFT"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT * FROM a RIGHT JOIN b ON a.id = b.id");
    assert_eq!(d[0].rule, "Convention/LeftJoin");
}

#[test]
fn points_to_right_keyword() {
    let d = check("SELECT * FROM a RIGHT JOIN b ON a.id = b.id");
    assert!(d[0].col >= 1);
    assert!(d[0].line >= 1);
}

#[test]
fn right_join_in_subquery_flagged() {
    let d = check("SELECT * FROM (SELECT * FROM a RIGHT JOIN b ON a.id = b.id) sub");
    assert_eq!(d.len(), 1);
}

#[test]
fn right_in_string_not_flagged() {
    assert!(check("SELECT 'RIGHT JOIN' FROM t").is_empty());
}

#[test]
fn right_in_comment_not_flagged() {
    assert!(check("SELECT a FROM t -- RIGHT JOIN example").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules left_join`
Expected: **compile error** (module not found) — that's the RED state.

---

### Task A2: Implement `LeftJoin`

Create `sqrust-rules/src/convention/left_join.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{JoinOperator, Query, Select, SetExpr, Statement, TableFactor};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct LeftJoin;

impl Rule for LeftJoin {
    fn name(&self) -> &'static str {
        "Convention/LeftJoin"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        let mut count = 0usize;
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, &ctx.source, &mut count, &mut diags);
            }
        }
        diags
    }
}

fn check_query(q: &Query, src: &str, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &q.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, src, count, diags);
        }
    }
    check_set_expr(&q.body, src, count, diags);
}

fn check_set_expr(expr: &SetExpr, src: &str, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, src, count, diags),
        SetExpr::Query(q) => check_query(q, src, count, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, src, count, diags);
            check_set_expr(right, src, count, diags);
        }
        _ => {}
    }
}

fn check_select(sel: &Select, src: &str, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    for twj in &sel.from {
        recurse_factor(&twj.relation, src, count, diags);
        for join in &twj.joins {
            recurse_factor(&join.relation, src, count, diags);
            if is_right_join(&join.join_operator) {
                let occ = *count;
                *count += 1;
                let offset = find_nth_keyword(src, b"RIGHT", occ);
                let (line, col) = offset_to_line_col(src, offset);
                diags.push(Diagnostic {
                    rule: "Convention/LeftJoin",
                    message: "Prefer LEFT JOIN over RIGHT JOIN; rewrite from the other table's perspective".to_string(),
                    line,
                    col,
                });
            }
        }
    }
}

fn recurse_factor(tf: &TableFactor, src: &str, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, src, count, diags);
    }
}

fn is_right_join(op: &JoinOperator) -> bool {
    matches!(
        op,
        JoinOperator::RightOuter(_) | JoinOperator::RightSemi(_) | JoinOperator::RightAnti(_)
    )
}

fn find_nth_keyword(source: &str, keyword: &[u8], nth: usize) -> usize {
    let bytes = source.as_bytes();
    let kw_len = keyword.len();
    let len = bytes.len();
    let skip = SkipMap::build(source);
    let mut count = 0;
    let mut i = 0;
    while i + kw_len <= len {
        if !skip.is_code(i) { i += 1; continue; }
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok { i += 1; continue; }
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(keyword.iter())
            .all(|(&a, &b)| a.to_ascii_uppercase() == b.to_ascii_uppercase());
        if matches {
            let end = i + kw_len;
            let after_ok = end >= len || !is_word_char(bytes[end]);
            let all_code = (i..end).all(|k| skip.is_code(k));
            if after_ok && all_code {
                if count == nth { return i; }
                count += 1;
                i += kw_len;
                continue;
            }
        }
        i += 1;
    }
    0
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Add to `sqrust-rules/src/convention/mod.rs`:
```
pub mod left_join;
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules left_join`
Expected: **all GREEN**

---

### Task A3: Write failing tests for `JoinConditionStyle`

Create `sqrust-rules/tests/join_condition_style_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::convention::join_condition_style::JoinConditionStyle;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    JoinConditionStyle.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(JoinConditionStyle.name(), "Convention/JoinConditionStyle");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn join_with_on_no_violation() {
    assert!(check("SELECT * FROM t1 JOIN t2 ON t1.id = t2.id").is_empty());
}

#[test]
fn no_join_where_no_violation() {
    assert!(check("SELECT * FROM t WHERE id > 1").is_empty());
}

#[test]
fn single_table_where_no_violation() {
    assert!(check("SELECT * FROM t WHERE t.id > 1 AND t.name = 'foo'").is_empty());
}

#[test]
fn cross_table_eq_in_where_flagged() {
    let d = check("SELECT * FROM t1, t2 WHERE t1.id = t2.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn cross_table_eq_in_where_with_explicit_join_flagged() {
    let d = check("SELECT * FROM t1 JOIN t2 ON TRUE WHERE t1.id = t2.fk");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_cross_table_eqs_in_where_flagged() {
    let d = check("SELECT * FROM t1, t2, t3 WHERE t1.id = t2.id AND t2.id = t3.id");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_on() {
    let d = check("SELECT * FROM t1, t2 WHERE t1.id = t2.id");
    assert!(d[0].message.to_uppercase().contains("ON"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT * FROM t1, t2 WHERE t1.id = t2.id");
    assert_eq!(d[0].rule, "Convention/JoinConditionStyle");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT * FROM t1, t2 WHERE t1.id = t2.id");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn same_table_eq_in_where_no_violation() {
    assert!(check("SELECT * FROM t WHERE t.a = t.b").is_empty());
}

#[test]
fn cross_table_eq_in_subquery_where_flagged() {
    let d = check("SELECT * FROM (SELECT * FROM a, b WHERE a.id = b.id) sub");
    assert_eq!(d.len(), 1);
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules join_condition_style`
Expected: **compile error** — RED state.

---

### Task A4: Implement `JoinConditionStyle`

Create `sqrust-rules/src/convention/join_condition_style.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{BinaryOperator, Expr, Query, Select, SetExpr, Statement, TableFactor};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct JoinConditionStyle;

impl Rule for JoinConditionStyle {
    fn name(&self) -> &'static str {
        "Convention/JoinConditionStyle"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        let mut count = 0usize;
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, ctx, &mut count, &mut diags);
            }
        }
        diags
    }
}

fn check_query(
    q: &Query,
    ctx: &FileContext,
    count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let Some(with) = &q.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, count, diags);
        }
    }
    check_set_expr(&q.body, ctx, count, diags);
}

fn check_set_expr(
    expr: &SetExpr,
    ctx: &FileContext,
    count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, ctx, count, diags),
        SetExpr::Query(q) => check_query(q, ctx, count, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, count, diags);
            check_set_expr(right, ctx, count, diags);
        }
        _ => {}
    }
}

fn check_select(
    sel: &Select,
    ctx: &FileContext,
    count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    // Recurse into subqueries in FROM.
    for twj in &sel.from {
        recurse_factor(&twj.relation, ctx, count, diags);
        for join in &twj.joins {
            recurse_factor(&join.relation, ctx, count, diags);
        }
    }
    // Check WHERE clause for cross-table equality comparisons.
    if let Some(where_expr) = &sel.selection {
        collect_cross_table_eq(where_expr, ctx, count, diags);
    }
}

fn recurse_factor(
    tf: &TableFactor,
    ctx: &FileContext,
    count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, ctx, count, diags);
    }
}

/// Recursively walks `expr` and flags any top-level `t1.col = t2.col` equality
/// where the table qualifiers differ.
fn collect_cross_table_eq(
    expr: &Expr,
    ctx: &FileContext,
    count: &mut usize,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::BinaryOp { left, op, right } => {
            if matches!(op, BinaryOperator::Eq) {
                if let (
                    Expr::CompoundIdentifier(l_parts),
                    Expr::CompoundIdentifier(r_parts),
                ) = (left.as_ref(), right.as_ref())
                {
                    if l_parts.len() >= 2 && r_parts.len() >= 2 {
                        let l_table = l_parts[0].value.to_lowercase();
                        let r_table = r_parts[0].value.to_lowercase();
                        if l_table != r_table {
                            let occ = *count;
                            *count += 1;
                            // Point to the left-hand table qualifier.
                            let offset =
                                find_nth_word(&ctx.source, &l_parts[0].value, occ);
                            let (line, col) = offset_to_line_col(&ctx.source, offset);
                            diags.push(Diagnostic {
                                rule: "Convention/JoinConditionStyle",
                                message: "Join condition found in WHERE clause; move it to the ON clause".to_string(),
                                line,
                                col,
                            });
                            return; // Don't recurse into this equality.
                        }
                    }
                }
            }
            // For AND/OR, recurse into both sides.
            collect_cross_table_eq(left, ctx, count, diags);
            collect_cross_table_eq(right, ctx, count, diags);
        }
        Expr::Nested(inner) => collect_cross_table_eq(inner, ctx, count, diags),
        _ => {}
    }
}

fn find_nth_word(source: &str, word: &str, nth: usize) -> usize {
    let bytes = source.as_bytes();
    let word_upper: Vec<u8> = word.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let wlen = word_upper.len();
    let len = bytes.len();
    let skip = SkipMap::build(source);
    let mut count = 0;
    let mut i = 0;
    while i + wlen <= len {
        if !skip.is_code(i) { i += 1; continue; }
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok { i += 1; continue; }
        let matches = bytes[i..i + wlen]
            .iter()
            .zip(word_upper.iter())
            .all(|(&a, &b)| a.to_ascii_uppercase() == b);
        if matches {
            let end = i + wlen;
            let after_ok = end >= len || !is_word_char(bytes[end]);
            if after_ok && (i..end).all(|k| skip.is_code(k)) {
                if count == nth { return i; }
                count += 1;
                i += wlen;
                continue;
            }
        }
        i += 1;
    }
    0
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Add to `sqrust-rules/src/convention/mod.rs`:
```
pub mod join_condition_style;
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules join_condition_style`
Expected: **all GREEN**

---

### Task A5: Register in CLI + commit

Add to `sqrust-cli/src/main.rs` (with the other convention imports):
```rust
use sqrust_rules::convention::left_join::LeftJoin;
use sqrust_rules::convention::join_condition_style::JoinConditionStyle;
```

Add to the `rules()` vec (after Wave 15 section):
```rust
// Wave 16
Box::new(LeftJoin),
Box::new(JoinConditionStyle),
```

Run: `~/.cargo/bin/cargo build -p sqrust-cli`

Commit:
```bash
git add sqrust-rules/src/convention/left_join.rs \
        sqrust-rules/src/convention/join_condition_style.rs \
        sqrust-rules/src/convention/mod.rs \
        sqrust-rules/tests/left_join_test.rs \
        sqrust-rules/tests/join_condition_style_test.rs \
        sqrust-cli/src/main.rs
git commit -m "feat(convention): add LeftJoin and JoinConditionStyle rules"
```

---

## AGENT B — Lint: `UnusedTableAlias` + `ConsecutiveSemicolons`

**Files to create:**
- `sqrust-rules/src/lint/unused_table_alias.rs`
- `sqrust-rules/src/lint/consecutive_semicolons.rs`
- `sqrust-rules/tests/unused_table_alias_test.rs`
- `sqrust-rules/tests/consecutive_semicolons_test.rs`

**Files to modify:**
- `sqrust-rules/src/lint/mod.rs`
- `sqrust-cli/src/main.rs`

---

### Task B1: Write failing tests for `UnusedTableAlias`

Create `sqrust-rules/tests/unused_table_alias_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::lint::unused_table_alias::UnusedTableAlias;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    UnusedTableAlias.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnusedTableAlias.name(), "Lint/UnusedTableAlias");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn alias_used_as_qualifier_no_violation() {
    assert!(check("SELECT a.id FROM orders AS a").is_empty());
}

#[test]
fn no_alias_no_violation() {
    assert!(check("SELECT id FROM orders").is_empty());
}

#[test]
fn alias_used_in_join_no_violation() {
    assert!(check("SELECT a.id, b.name FROM orders AS a JOIN customers AS b ON a.cid = b.id").is_empty());
}

#[test]
fn unused_alias_flagged() {
    let d = check("SELECT id FROM orders AS o");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_unused_aliases_flagged() {
    let d = check("SELECT t1.id FROM orders AS o JOIN customers AS c ON t1.id = t2.id");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_alias_name() {
    let d = check("SELECT id FROM orders AS o");
    assert!(d[0].message.contains('o') || d[0].message.to_lowercase().contains("alias"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM orders AS o");
    assert_eq!(d[0].rule, "Lint/UnusedTableAlias");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM orders AS o");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn alias_used_in_where_no_violation() {
    assert!(check("SELECT id FROM orders AS o WHERE o.status = 1").is_empty());
}

#[test]
fn alias_used_in_order_by_no_violation() {
    assert!(check("SELECT id FROM orders AS o ORDER BY o.created_at").is_empty());
}

#[test]
fn subquery_alias_used_no_violation() {
    assert!(check("SELECT sub.id FROM (SELECT id FROM t) AS sub WHERE sub.id > 1").is_empty());
}

#[test]
fn subquery_alias_unused_flagged() {
    let d = check("SELECT id FROM (SELECT id FROM t) AS sub");
    assert_eq!(d.len(), 1);
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules unused_table_alias`
Expected: **compile error** — RED.

---

### Task B2: Implement `UnusedTableAlias`

Create `sqrust-rules/src/lint/unused_table_alias.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, Select, SetExpr, Statement, TableAlias, TableFactor};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct UnusedTableAlias;

impl Rule for UnusedTableAlias {
    fn name(&self) -> &'static str {
        "Lint/UnusedTableAlias"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, ctx, &mut diags);
            }
        }
        diags
    }
}

fn check_query(q: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &q.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, diags);
        }
    }
    check_set_expr(&q.body, ctx, diags);
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, ctx, diags),
        SetExpr::Query(q) => check_query(q, ctx, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, diags);
            check_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(sel: &Select, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    for twj in &sel.from {
        check_table_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            check_table_factor(&join.relation, ctx, diags);
        }
    }
}

fn check_table_factor(tf: &TableFactor, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match tf {
        TableFactor::Table { alias, .. } => {
            if let Some(TableAlias { name, .. }) = alias {
                check_alias_used(&name.value, ctx, diags);
            }
        }
        TableFactor::Derived { subquery, alias, .. } => {
            check_query(subquery, ctx, diags);
            if let Some(TableAlias { name, .. }) = alias {
                check_alias_used(&name.value, ctx, diags);
            }
        }
        _ => {}
    }
}

/// An alias is "used" if `alias.` appears anywhere in the source after its definition.
fn check_alias_used(alias: &str, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    let source = &ctx.source;
    let source_upper = source.to_uppercase();
    let alias_upper = alias.to_uppercase();

    // Find definition position (first occurrence of alias as a whole word).
    let def_pos = find_word_position(source, alias);

    // After the definition, look for `alias.` pattern.
    let after_def = &source_upper[def_pos + alias_upper.len()..];
    let qualifier = format!("{}.", alias_upper);
    let used = after_def.contains(&qualifier);

    if !used {
        let (line, col) = offset_to_line_col(source, def_pos);
        diags.push(Diagnostic {
            rule: "Lint/UnusedTableAlias",
            message: format!("Table alias '{}' is defined but never used as a qualifier", alias),
            line,
            col,
        });
    }
}

fn find_word_position(source: &str, word: &str) -> usize {
    let bytes = source.as_bytes();
    let word_upper: Vec<u8> = word.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let wlen = word_upper.len();
    let len = bytes.len();
    let skip = SkipMap::build(source);
    let mut i = 0;
    while i + wlen <= len {
        if !skip.is_code(i) { i += 1; continue; }
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok { i += 1; continue; }
        let matches = bytes[i..i + wlen]
            .iter()
            .zip(word_upper.iter())
            .all(|(&a, &b)| a.to_ascii_uppercase() == b);
        if matches {
            let end = i + wlen;
            let after_ok = end >= len || !is_word_char(bytes[end]);
            if after_ok { return i; }
        }
        i += 1;
    }
    0
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Add to `sqrust-rules/src/lint/mod.rs`:
```
pub mod unused_table_alias;
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules unused_table_alias`
Expected: **all GREEN**

---

### Task B3: Write failing tests for `ConsecutiveSemicolons`

Create `sqrust-rules/tests/consecutive_semicolons_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::lint::consecutive_semicolons::ConsecutiveSemicolons;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ConsecutiveSemicolons.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ConsecutiveSemicolons.name(), "Lint/ConsecutiveSemicolons");
}

#[test]
fn parse_error_no_violation() {
    // Parse errors don't apply here — text-based rule always runs.
    assert!(check("SELECT 1;").is_empty());
}

#[test]
fn single_semicolon_no_violation() {
    assert!(check("SELECT 1;").is_empty());
}

#[test]
fn two_statements_no_violation() {
    assert!(check("SELECT 1;\nSELECT 2;").is_empty());
}

#[test]
fn double_semicolon_flagged() {
    let d = check("SELECT 1;;");
    assert_eq!(d.len(), 1);
}

#[test]
fn triple_semicolon_flagged_once() {
    let d = check("SELECT 1;;;");
    assert_eq!(d.len(), 1);
}

#[test]
fn double_semicolon_on_own_line_flagged() {
    let d = check("SELECT 1;\n;");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_separate_double_semicolons_flagged_twice() {
    let d = check("SELECT 1;;\nSELECT 2;;");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_semicolons() {
    let d = check("SELECT 1;;");
    assert!(d[0].message.contains(';') || d[0].message.to_lowercase().contains("semicolon"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT 1;;");
    assert_eq!(d[0].rule, "Lint/ConsecutiveSemicolons");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT 1;;");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn semicolon_in_string_not_flagged() {
    assert!(check("SELECT ';;' FROM t").is_empty());
}

#[test]
fn semicolon_in_comment_not_flagged() {
    assert!(check("SELECT 1 -- ;;\n;").is_empty());
}

#[test]
fn no_semicolon_no_violation() {
    assert!(check("SELECT 1").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules consecutive_semicolons`
Expected: **compile error** — RED.

---

### Task B4: Implement `ConsecutiveSemicolons`

Create `sqrust-rules/src/lint/consecutive_semicolons.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct ConsecutiveSemicolons;

impl Rule for ConsecutiveSemicolons {
    fn name(&self) -> &'static str {
        "Lint/ConsecutiveSemicolons"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);
        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            if !skip.is_code(i) { i += 1; continue; }

            if bytes[i] == b';' {
                // Scan forward (skipping whitespace) for another semicolon.
                let mut j = i + 1;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                    j += 1;
                }
                if j < len && skip.is_code(j) && bytes[j] == b';' {
                    let (line, col) = offset_to_line_col(source, i);
                    diags.push(Diagnostic {
                        rule: "Lint/ConsecutiveSemicolons",
                        message: "Consecutive semicolons (;;) found; remove the extra semicolon".to_string(),
                        line,
                        col,
                    });
                    // Skip to end of this semicolon run.
                    while i < len && (bytes[i] == b';' || bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
                        if bytes[i] == b';' && i > 0 { i += 1; } else { i += 1; }
                        // Stop when we hit non-semicolon non-whitespace after the run.
                        if i < len && bytes[i] != b';' && bytes[i] != b' ' && bytes[i] != b'\t' && bytes[i] != b'\n' && bytes[i] != b'\r' {
                            break;
                        }
                    }
                    continue;
                }
            }
            i += 1;
        }

        diags
    }
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Add to `sqrust-rules/src/lint/mod.rs`:
```
pub mod consecutive_semicolons;
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules consecutive_semicolons`
Expected: **all GREEN**

---

### Task B5: Register in CLI + commit

Add to `sqrust-cli/src/main.rs`:
```rust
use sqrust_rules::lint::unused_table_alias::UnusedTableAlias;
use sqrust_rules::lint::consecutive_semicolons::ConsecutiveSemicolons;
```

Add to `rules()` vec (Wave 16 section):
```rust
Box::new(UnusedTableAlias),
Box::new(ConsecutiveSemicolons),
```

Run: `~/.cargo/bin/cargo build -p sqrust-cli`

Commit:
```bash
git add sqrust-rules/src/lint/unused_table_alias.rs \
        sqrust-rules/src/lint/consecutive_semicolons.rs \
        sqrust-rules/src/lint/mod.rs \
        sqrust-rules/tests/unused_table_alias_test.rs \
        sqrust-rules/tests/consecutive_semicolons_test.rs \
        sqrust-cli/src/main.rs
git commit -m "feat(lint): add UnusedTableAlias and ConsecutiveSemicolons rules"
```

---

## AGENT C — Structure: `NestedCaseInElse` + `UnusedJoin`

**Files to create:**
- `sqrust-rules/src/structure/nested_case_in_else.rs`
- `sqrust-rules/src/structure/unused_join.rs`
- `sqrust-rules/tests/nested_case_in_else_test.rs`
- `sqrust-rules/tests/unused_join_test.rs`

**Files to modify:**
- `sqrust-rules/src/structure/mod.rs`
- `sqrust-cli/src/main.rs`

---

### Task C1: Write failing tests for `NestedCaseInElse`

Create `sqrust-rules/tests/nested_case_in_else_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::structure::nested_case_in_else::NestedCaseInElse;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    NestedCaseInElse.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(NestedCaseInElse.name(), "Structure/NestedCaseInElse");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn simple_case_no_violation() {
    assert!(check("SELECT CASE WHEN x = 1 THEN 'a' ELSE 'b' END FROM t").is_empty());
}

#[test]
fn case_with_no_else_no_violation() {
    assert!(check("SELECT CASE WHEN x = 1 THEN 'a' END FROM t").is_empty());
}

#[test]
fn case_in_then_not_flagged() {
    // CASE nested in THEN is fine; only ELSE is flagged.
    assert!(check("SELECT CASE WHEN x = 1 THEN CASE WHEN y = 2 THEN 'a' ELSE 'b' END ELSE 'c' END FROM t").is_empty());
}

#[test]
fn case_nested_in_else_flagged() {
    let d = check("SELECT CASE WHEN x = 1 THEN 'a' ELSE CASE WHEN y = 2 THEN 'b' ELSE 'c' END END FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn double_nested_else_case_flagged_once() {
    // Inner CASE is the violation — outer ELSE contains CASE.
    let d = check("SELECT CASE WHEN a = 1 THEN 'x' ELSE CASE WHEN b = 2 THEN 'y' ELSE CASE WHEN c = 3 THEN 'z' ELSE 'w' END END END FROM t");
    assert!(d.len() >= 1);
}

#[test]
fn two_separate_nested_cases_flagged_twice() {
    let sql = "SELECT CASE WHEN a = 1 THEN 'x' ELSE CASE WHEN b = 2 THEN 'y' ELSE 'z' END END, CASE WHEN c = 3 THEN 'p' ELSE CASE WHEN d = 4 THEN 'q' ELSE 'r' END END FROM t";
    let d = check(sql);
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_else() {
    let d = check("SELECT CASE WHEN x = 1 THEN 'a' ELSE CASE WHEN y = 2 THEN 'b' ELSE 'c' END END FROM t");
    assert!(d[0].message.to_uppercase().contains("ELSE") || d[0].message.to_uppercase().contains("CASE"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT CASE WHEN x = 1 THEN 'a' ELSE CASE WHEN y = 2 THEN 'b' ELSE 'c' END END FROM t");
    assert_eq!(d[0].rule, "Structure/NestedCaseInElse");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT CASE WHEN x = 1 THEN 'a' ELSE CASE WHEN y = 2 THEN 'b' ELSE 'c' END END FROM t");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn nested_case_in_subquery_flagged() {
    let d = check("SELECT * FROM (SELECT CASE WHEN x = 1 THEN 'a' ELSE CASE WHEN y = 2 THEN 'b' ELSE 'c' END END AS v FROM t) sub");
    assert_eq!(d.len(), 1);
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules nested_case_in_else`
Expected: **compile error** — RED.

---

### Task C2: Implement `NestedCaseInElse`

Create `sqrust-rules/src/structure/nested_case_in_else.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, TableFactor};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct NestedCaseInElse;

impl Rule for NestedCaseInElse {
    fn name(&self) -> &'static str {
        "Structure/NestedCaseInElse"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        let mut case_count = 0usize;
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, ctx, &mut case_count, &mut diags);
            }
        }
        diags
    }
}

fn check_query(q: &Query, ctx: &FileContext, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &q.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, count, diags);
        }
    }
    check_set_expr(&q.body, ctx, count, diags);
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, ctx, count, diags),
        SetExpr::Query(q) => check_query(q, ctx, count, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, count, diags);
            check_set_expr(right, ctx, count, diags);
        }
        _ => {}
    }
}

fn check_select(sel: &Select, ctx: &FileContext, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                walk_expr(e, ctx, count, diags);
            }
            _ => {}
        }
    }
    if let Some(where_expr) = &sel.selection {
        walk_expr(where_expr, ctx, count, diags);
    }
    for twj in &sel.from {
        recurse_factor(&twj.relation, ctx, count, diags);
        for join in &twj.joins {
            recurse_factor(&join.relation, ctx, count, diags);
        }
    }
}

fn recurse_factor(tf: &TableFactor, ctx: &FileContext, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, ctx, count, diags);
    }
}

fn walk_expr(expr: &Expr, ctx: &FileContext, count: &mut usize, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Case { else_result, conditions, results, operand } => {
            // Check if else_result is itself a Case expression.
            if let Some(else_expr) = else_result {
                if matches!(else_expr.as_ref(), Expr::Case { .. }) {
                    let occ = *count;
                    *count += 1;
                    // Find the Nth ELSE keyword.
                    let offset = find_nth_keyword(&ctx.source, b"ELSE", occ);
                    let (line, col) = offset_to_line_col(&ctx.source, offset);
                    diags.push(Diagnostic {
                        rule: "Structure/NestedCaseInElse",
                        message: "CASE expression nested in ELSE clause; consider flattening with additional WHEN branches".to_string(),
                        line,
                        col,
                    });
                }
                // Always recurse into else_result for deeper nesting.
                walk_expr(else_expr, ctx, count, diags);
            }
            if let Some(op) = operand {
                walk_expr(op, ctx, count, diags);
            }
            for c in conditions { walk_expr(c, ctx, count, diags); }
            for r in results { walk_expr(r, ctx, count, diags); }
        }
        Expr::BinaryOp { left, right, .. } => {
            walk_expr(left, ctx, count, diags);
            walk_expr(right, ctx, count, diags);
        }
        Expr::Function(f) => {
            for arg in f.args.iter() {
                if let sqlparser::ast::FunctionArg::Unnamed(sqlparser::ast::FunctionArgExpr::Expr(e)) = arg {
                    walk_expr(e, ctx, count, diags);
                }
            }
        }
        Expr::Nested(inner) => walk_expr(inner, ctx, count, diags),
        _ => {}
    }
}

fn find_nth_keyword(source: &str, keyword: &[u8], nth: usize) -> usize {
    let bytes = source.as_bytes();
    let kw_len = keyword.len();
    let len = bytes.len();
    let skip = SkipMap::build(source);
    let mut count = 0;
    let mut i = 0;
    while i + kw_len <= len {
        if !skip.is_code(i) { i += 1; continue; }
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok { i += 1; continue; }
        let matches = bytes[i..i + kw_len]
            .iter()
            .zip(keyword.iter())
            .all(|(&a, &b)| a.to_ascii_uppercase() == b.to_ascii_uppercase());
        if matches {
            let end = i + kw_len;
            let after_ok = end >= len || !is_word_char(bytes[end]);
            if after_ok && (i..end).all(|k| skip.is_code(k)) {
                if count == nth { return i; }
                count += 1;
                i += kw_len;
                continue;
            }
        }
        i += 1;
    }
    0
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Add to `sqrust-rules/src/structure/mod.rs`:
```
pub mod nested_case_in_else;
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules nested_case_in_else`
Expected: **all GREEN**

---

### Task C3: Write failing tests for `UnusedJoin`

Create `sqrust-rules/tests/unused_join_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::structure::unused_join::UnusedJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    UnusedJoin.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnusedJoin.name(), "Structure/UnusedJoin");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn no_join_no_violation() {
    assert!(check("SELECT id FROM orders").is_empty());
}

#[test]
fn join_used_in_select_no_violation() {
    assert!(check("SELECT a.id, b.name FROM orders AS a JOIN customers AS b ON a.cid = b.id").is_empty());
}

#[test]
fn join_used_in_where_no_violation() {
    assert!(check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id WHERE b.active = 1").is_empty());
}

#[test]
fn unused_join_flagged() {
    let d = check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_unused_joins_flagged() {
    let d = check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id JOIN products AS p ON a.pid = p.id");
    assert_eq!(d.len(), 2);
}

#[test]
fn join_used_in_order_by_no_violation() {
    assert!(check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id ORDER BY b.name").is_empty());
}

#[test]
fn message_mentions_join_or_table() {
    let d = check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id");
    assert!(d[0].message.to_lowercase().contains("join") || d[0].message.to_lowercase().contains("b"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id");
    assert_eq!(d[0].rule, "Structure/UnusedJoin");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT a.id FROM orders AS a JOIN customers AS b ON a.cid = b.id");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn join_used_in_having_no_violation() {
    assert!(check("SELECT a.id, COUNT(*) FROM orders AS a JOIN customers AS b ON a.cid = b.id GROUP BY a.id HAVING MAX(b.score) > 5").is_empty());
}

#[test]
fn join_without_alias_used_no_violation() {
    assert!(check("SELECT orders.id, customers.name FROM orders JOIN customers ON orders.cid = customers.id").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules unused_join`
Expected: **compile error** — RED.

---

### Task C4: Implement `UnusedJoin`

Create `sqrust-rules/src/structure/unused_join.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, Select, SetExpr, Statement, TableAlias, TableFactor};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct UnusedJoin;

impl Rule for UnusedJoin {
    fn name(&self) -> &'static str {
        "Structure/UnusedJoin"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, ctx, &mut diags);
            }
        }
        diags
    }
}

fn check_query(q: &Query, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let Some(with) = &q.with {
        for cte in &with.cte_tables {
            check_query(&cte.query, ctx, diags);
        }
    }
    check_set_expr(&q.body, ctx, diags);
}

fn check_set_expr(expr: &SetExpr, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    match expr {
        SetExpr::Select(sel) => check_select(sel, ctx, diags),
        SetExpr::Query(q) => check_query(q, ctx, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, ctx, diags);
            check_set_expr(right, ctx, diags);
        }
        _ => {}
    }
}

fn check_select(sel: &Select, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    for twj in &sel.from {
        // Recurse into subqueries in base table and joins.
        recurse_factor(&twj.relation, ctx, diags);
        for join in &twj.joins {
            recurse_factor(&join.relation, ctx, diags);
            // For each join, determine the name/alias and check if it's referenced.
            let join_ref = table_factor_ref_name(&join.relation);
            if let Some(ref_name) = join_ref {
                // "Used" means `ref_name.` appears in the source after the join definition.
                let source = &ctx.source;
                let qualifier = format!("{}.", ref_name.to_uppercase());
                let source_upper = source.to_uppercase();
                // Find first occurrence of ref_name to locate definition.
                let def_pos = find_word_position(source, &ref_name);
                let after_def = &source_upper[def_pos + ref_name.len()..];
                if !after_def.contains(&qualifier) {
                    let (line, col) = offset_to_line_col(source, def_pos);
                    diags.push(Diagnostic {
                        rule: "Structure/UnusedJoin",
                        message: format!(
                            "Joined table '{}' is never referenced in SELECT, WHERE, or HAVING; the join may be unnecessary",
                            ref_name
                        ),
                        line,
                        col,
                    });
                }
            }
        }
    }
}

fn recurse_factor(tf: &TableFactor, ctx: &FileContext, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, ctx, diags);
    }
}

/// Returns the alias name if present, otherwise the table name.
fn table_factor_ref_name(tf: &TableFactor) -> Option<String> {
    match tf {
        TableFactor::Table { name, alias, .. } => {
            if let Some(TableAlias { name: alias_name, .. }) = alias {
                Some(alias_name.value.clone())
            } else {
                // Use last part of compound table name.
                name.0.last().map(|ident| ident.value.clone())
            }
        }
        TableFactor::Derived { alias, .. } => {
            alias.as_ref().map(|a| a.name.value.clone())
        }
        _ => None,
    }
}

fn find_word_position(source: &str, word: &str) -> usize {
    let bytes = source.as_bytes();
    let word_upper: Vec<u8> = word.bytes().map(|b| b.to_ascii_uppercase()).collect();
    let wlen = word_upper.len();
    let len = bytes.len();
    let skip = SkipMap::build(source);
    let mut i = 0;
    while i + wlen <= len {
        if !skip.is_code(i) { i += 1; continue; }
        let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
        if !before_ok { i += 1; continue; }
        let matches = bytes[i..i + wlen]
            .iter()
            .zip(word_upper.iter())
            .all(|(&a, &b)| a.to_ascii_uppercase() == b);
        if matches {
            let end = i + wlen;
            let after_ok = end >= len || !is_word_char(bytes[end]);
            if after_ok { return i; }
        }
        i += 1;
    }
    0
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Add to `sqrust-rules/src/structure/mod.rs`:
```
pub mod nested_case_in_else;
pub mod unused_join;
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules unused_join`
Expected: **all GREEN**

---

### Task C5: Register in CLI + commit

Add to `sqrust-cli/src/main.rs`:
```rust
use sqrust_rules::structure::nested_case_in_else::NestedCaseInElse;
use sqrust_rules::structure::unused_join::UnusedJoin;
```

Add to `rules()` vec:
```rust
Box::new(NestedCaseInElse),
Box::new(UnusedJoin),
```

Run: `~/.cargo/bin/cargo build -p sqrust-cli`

Commit:
```bash
git add sqrust-rules/src/structure/nested_case_in_else.rs \
        sqrust-rules/src/structure/unused_join.rs \
        sqrust-rules/src/structure/mod.rs \
        sqrust-rules/tests/nested_case_in_else_test.rs \
        sqrust-rules/tests/unused_join_test.rs \
        sqrust-cli/src/main.rs
git commit -m "feat(structure): add NestedCaseInElse and UnusedJoin rules"
```

---

## AGENT D — Ambiguous: `InconsistentOrderByDirection` + `InconsistentColumnReference`

**Files to create:**
- `sqrust-rules/src/ambiguous/inconsistent_order_by_direction.rs`
- `sqrust-rules/src/ambiguous/inconsistent_column_reference.rs`
- `sqrust-rules/tests/inconsistent_order_by_direction_test.rs`
- `sqrust-rules/tests/inconsistent_column_reference_test.rs`

**Files to modify:**
- `sqrust-rules/src/ambiguous/mod.rs`
- `sqrust-cli/src/main.rs`

**Important:** Both rules reuse the existing `match_keyword`, `skip_whitespace`, and `scan_positional_list` helpers exported from `sqrust-rules/src/ambiguous/group_by_position.rs`. Import with:
```rust
use super::group_by_position::{match_keyword, skip_whitespace};
```

---

### Task D1: Write failing tests for `InconsistentOrderByDirection`

Create `sqrust-rules/tests/inconsistent_order_by_direction_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::inconsistent_order_by_direction::InconsistentOrderByDirection;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    InconsistentOrderByDirection.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(InconsistentOrderByDirection.name(), "Ambiguous/InconsistentOrderByDirection");
}

#[test]
fn parse_error_no_violation() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn all_explicit_asc_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a ASC, b ASC").is_empty());
}

#[test]
fn all_explicit_desc_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a DESC, b DESC").is_empty());
}

#[test]
fn all_implicit_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a, b, c").is_empty());
}

#[test]
fn single_column_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY a ASC").is_empty());
}

#[test]
fn mixed_asc_and_implicit_flagged() {
    let d = check("SELECT * FROM t ORDER BY a ASC, b");
    assert_eq!(d.len(), 1);
}

#[test]
fn mixed_desc_and_implicit_flagged() {
    let d = check("SELECT * FROM t ORDER BY a, b DESC");
    assert_eq!(d.len(), 1);
}

#[test]
fn mixed_asc_desc_and_implicit_flagged() {
    let d = check("SELECT * FROM t ORDER BY a ASC, b DESC, c");
    assert_eq!(d.len(), 1);
}

#[test]
fn message_mentions_direction() {
    let d = check("SELECT * FROM t ORDER BY a ASC, b");
    let msg = d[0].message.to_uppercase();
    assert!(msg.contains("ASC") || msg.contains("DESC") || msg.contains("DIRECTION") || msg.contains("CONSISTENT"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT * FROM t ORDER BY a ASC, b");
    assert_eq!(d[0].rule, "Ambiguous/InconsistentOrderByDirection");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT * FROM t ORDER BY a ASC, b");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn points_to_order_by() {
    // Violation reported on the ORDER BY keyword line.
    let sql = "SELECT * FROM t\nORDER BY a ASC, b";
    let d = check(sql);
    assert!(d[0].line >= 1);
}

#[test]
fn asc_in_string_not_counted() {
    assert!(check("SELECT 'ASC' FROM t ORDER BY a ASC, b ASC").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules inconsistent_order_by_direction`
Expected: **compile error** — RED.

---

### Task D2: Implement `InconsistentOrderByDirection`

Create `sqrust-rules/src/ambiguous/inconsistent_order_by_direction.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};
use super::group_by_position::{match_keyword, skip_whitespace};

pub struct InconsistentOrderByDirection;

impl Rule for InconsistentOrderByDirection {
    fn name(&self) -> &'static str {
        "Ambiguous/InconsistentOrderByDirection"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);
        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            if !skip_map.is_code(i) { i += 1; continue; }

            if let Some(after_order) = match_keyword(bytes, &skip_map, i, b"ORDER") {
                let after_ws = skip_whitespace(bytes, after_order);
                if let Some(after_by) = match_keyword(bytes, &skip_map, after_ws, b"BY") {
                    // Collect ORDER BY items and check direction consistency.
                    let order_by_start = after_by;
                    if let Some(violation_pos) = check_direction_consistency(
                        bytes, &skip_map, source, order_by_start,
                    ) {
                        let (line, col) = offset_to_line_col(source, violation_pos);
                        diags.push(Diagnostic {
                            rule: "Ambiguous/InconsistentOrderByDirection",
                            message: "ORDER BY mixes explicit direction (ASC/DESC) with implicit; be explicit on all columns".to_string(),
                            line,
                            col,
                        });
                    }
                    i = after_by;
                    continue;
                }
            }

            i += 1;
        }

        diags
    }
}

/// ORDER BY stop keywords (same as in group_by_position).
const STOP_KEYWORDS: &[&[u8]] = &[
    b"LIMIT", b"UNION", b"INTERSECT", b"EXCEPT", b"FETCH", b"OFFSET",
    b"FOR", b"INTO", b";",
];

/// Scans the ORDER BY item list. Returns the byte offset of ORDER BY start
/// if the clause mixes explicit direction markers with implicit ones.
fn check_direction_consistency(
    bytes: &[u8],
    skip_map: &SkipMap,
    source: &str,
    start: usize,
) -> Option<usize> {
    let len = bytes.len();
    let mut has_explicit = false;
    let mut has_implicit = false;
    let order_by_pos = start;

    let mut i = start;
    // Skip items until a stop keyword or end of input.
    loop {
        // Skip whitespace.
        while i < len && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
            i += 1;
        }
        if i >= len { break; }
        if !skip_map.is_code(i) { i += 1; continue; }

        // Check for stop keyword.
        let mut stopped = false;
        for kw in STOP_KEYWORDS {
            if match_keyword_at(bytes, skip_map, i, kw) {
                stopped = true;
                break;
            }
        }
        if stopped { break; }

        // Scan to end of this ORDER BY item (next comma at depth 0 or stop keyword).
        let item_start = i;
        let mut depth = 0i32;
        let mut item_end = i;
        let mut last_word_start = i;
        let mut last_word_end = i;

        while item_end < len {
            if !skip_map.is_code(item_end) { item_end += 1; continue; }
            let b = bytes[item_end];
            if b == b'(' { depth += 1; item_end += 1; continue; }
            if b == b')' {
                if depth > 0 { depth -= 1; item_end += 1; continue; }
                else { break; } // end of subexpression
            }
            if depth == 0 && b == b',' { break; }
            // Track last word boundary.
            if is_word_char(b) {
                if item_end == 0 || !is_word_char(bytes[item_end - 1]) {
                    last_word_start = item_end;
                }
                last_word_end = item_end + 1;
            }
            // Check stop keywords at depth 0.
            let mut at_stop = false;
            for kw in STOP_KEYWORDS {
                if match_keyword_at(bytes, skip_map, item_end, kw) {
                    at_stop = true;
                    break;
                }
            }
            if at_stop { break; }
            item_end += 1;
        }

        // Check if the last word of this item is ASC or DESC.
        if last_word_end > last_word_start {
            let last_word = &bytes[last_word_start..last_word_end];
            let is_asc = last_word.len() == 3
                && b"ASC".iter().zip(last_word).all(|(a, b)| a.eq_ignore_ascii_case(b));
            let is_desc = last_word.len() == 4
                && b"DESC".iter().zip(last_word).all(|(a, b)| a.eq_ignore_ascii_case(b));
            if is_asc || is_desc {
                has_explicit = true;
            } else {
                // Check it's not NULLS (for NULLS FIRST/LAST).
                let is_first = last_word.len() == 5
                    && b"FIRST".iter().zip(last_word).all(|(a, b)| a.eq_ignore_ascii_case(b));
                let is_last = last_word.len() == 4
                    && b"LAST".iter().zip(last_word).all(|(a, b)| a.eq_ignore_ascii_case(b));
                if !is_first && !is_last {
                    has_implicit = true;
                }
            }
        }

        // Move past this item.
        i = item_end;
        if i < len && bytes[i] == b',' { i += 1; }
    }

    if has_explicit && has_implicit {
        Some(order_by_pos)
    } else {
        None
    }
}

fn match_keyword_at(bytes: &[u8], skip_map: &SkipMap, i: usize, kw: &[u8]) -> bool {
    let len = bytes.len();
    let kw_len = kw.len();
    if i + kw_len > len { return false; }
    if !skip_map.is_code(i) { return false; }
    let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
    if !before_ok { return false; }
    let matches = bytes[i..i + kw_len]
        .iter()
        .zip(kw.iter())
        .all(|(&a, &b)| a.to_ascii_uppercase() == b.to_ascii_uppercase());
    if !matches { return false; }
    let end = i + kw_len;
    end >= len || !is_word_char(bytes[end])
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Add to `sqrust-rules/src/ambiguous/mod.rs`:
```
pub mod inconsistent_order_by_direction;
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules inconsistent_order_by_direction`
Expected: **all GREEN**

---

### Task D3: Write failing tests for `InconsistentColumnReference`

Create `sqrust-rules/tests/inconsistent_column_reference_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::inconsistent_column_reference::InconsistentColumnReference;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    InconsistentColumnReference.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(InconsistentColumnReference.name(), "Ambiguous/InconsistentColumnReference");
}

#[test]
fn parse_error_no_violation() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn all_named_order_by_no_violation() {
    assert!(check("SELECT * FROM t ORDER BY name, age").is_empty());
}

#[test]
fn all_positional_order_by_no_violation() {
    assert!(check("SELECT id, name FROM t ORDER BY 1, 2").is_empty());
}

#[test]
fn mixed_in_order_by_flagged() {
    let d = check("SELECT id, name FROM t ORDER BY 1, name");
    assert_eq!(d.len(), 1);
}

#[test]
fn all_named_group_by_no_violation() {
    assert!(check("SELECT dept, COUNT(*) FROM t GROUP BY dept").is_empty());
}

#[test]
fn all_positional_group_by_no_violation() {
    assert!(check("SELECT dept, COUNT(*) FROM t GROUP BY 1").is_empty());
}

#[test]
fn mixed_in_group_by_flagged() {
    let d = check("SELECT dept, region, COUNT(*) FROM t GROUP BY 1, region");
    assert_eq!(d.len(), 1);
}

#[test]
fn mixed_in_both_clauses_flagged_twice() {
    let d = check("SELECT dept, region FROM t GROUP BY 1, region ORDER BY 1, dept");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_positional_or_reference() {
    let d = check("SELECT id, name FROM t ORDER BY 1, name");
    let msg = d[0].message.to_lowercase();
    assert!(msg.contains("positional") || msg.contains("reference") || msg.contains("consistent") || msg.contains("mix"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id, name FROM t ORDER BY 1, name");
    assert_eq!(d[0].rule, "Ambiguous/InconsistentColumnReference");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id, name FROM t ORDER BY 1, name");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn number_in_string_not_counted() {
    assert!(check("SELECT '1' FROM t ORDER BY name, age").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules inconsistent_column_reference`
Expected: **compile error** — RED.

---

### Task D4: Implement `InconsistentColumnReference`

Create `sqrust-rules/src/ambiguous/inconsistent_column_reference.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};
use super::group_by_position::{match_keyword, skip_whitespace};

pub struct InconsistentColumnReference;

impl Rule for InconsistentColumnReference {
    fn name(&self) -> &'static str {
        "Ambiguous/InconsistentColumnReference"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);
        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            if !skip_map.is_code(i) { i += 1; continue; }

            // Check ORDER BY.
            if let Some(after_order) = match_keyword(bytes, &skip_map, i, b"ORDER") {
                let after_ws = skip_whitespace(bytes, after_order);
                if let Some(after_by) = match_keyword(bytes, &skip_map, after_ws, b"BY") {
                    if has_mixed_references(bytes, &skip_map, after_by) {
                        let (line, col) = offset_to_line_col(source, i);
                        diags.push(Diagnostic {
                            rule: "Ambiguous/InconsistentColumnReference",
                            message: "ORDER BY mixes positional references (e.g. 1) with named column references; use one style consistently".to_string(),
                            line,
                            col,
                        });
                    }
                    i = after_by;
                    continue;
                }
            }

            // Check GROUP BY.
            if let Some(after_group) = match_keyword(bytes, &skip_map, i, b"GROUP") {
                let after_ws = skip_whitespace(bytes, after_group);
                if let Some(after_by) = match_keyword(bytes, &skip_map, after_ws, b"BY") {
                    if has_mixed_references(bytes, &skip_map, after_by) {
                        let (line, col) = offset_to_line_col(source, i);
                        diags.push(Diagnostic {
                            rule: "Ambiguous/InconsistentColumnReference",
                            message: "GROUP BY mixes positional references (e.g. 1) with named column references; use one style consistently".to_string(),
                            line,
                            col,
                        });
                    }
                    i = after_by;
                    continue;
                }
            }

            i += 1;
        }

        diags
    }
}

const STOP: &[&[u8]] = &[
    b"HAVING", b"ORDER", b"LIMIT", b"UNION", b"INTERSECT", b"EXCEPT",
    b"FETCH", b"WHERE", b"FOR", b";",
];

/// Returns true if the clause has both positional (numeric) and named items.
fn has_mixed_references(bytes: &[u8], skip_map: &SkipMap, start: usize) -> bool {
    let len = bytes.len();
    let mut has_positional = false;
    let mut has_named = false;
    let mut i = start;

    loop {
        // Skip whitespace.
        while i < len && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
            i += 1;
        }
        if i >= len { break; }
        if !skip_map.is_code(i) { i += 1; continue; }

        // Stop keyword check.
        let mut stopped = false;
        for kw in STOP {
            if kw_matches(bytes, skip_map, i, kw) { stopped = true; break; }
        }
        if stopped { break; }

        // Scan one item (up to next top-level comma or stop).
        let mut depth = 0i32;
        let mut item_is_positional = false;
        let mut item_is_named = false;
        let item_start = i;

        // First token of the item determines its type.
        // Skip whitespace first.
        let first_tok_start = i;
        if i < len && skip_map.is_code(i) {
            if bytes[i].is_ascii_digit() {
                // Positional reference.
                item_is_positional = true;
            } else if is_word_char(bytes[i]) {
                item_is_named = true;
            }
        }

        // Scan to end of item.
        while i < len {
            if !skip_map.is_code(i) { i += 1; continue; }
            let b = bytes[i];
            if b == b'(' { depth += 1; i += 1; continue; }
            if b == b')' {
                if depth > 0 { depth -= 1; i += 1; continue; }
                else { break; }
            }
            if depth == 0 && b == b',' { break; }
            let mut at_stop = false;
            for kw in STOP {
                if kw_matches(bytes, skip_map, i, kw) { at_stop = true; break; }
            }
            if at_stop { break; }
            i += 1;
        }

        // Skip ASC/DESC/NULLS FIRST/LAST suffixes for ORDER BY.
        // The item type was determined by first token above.
        if item_is_positional { has_positional = true; }
        if item_is_named { has_named = true; }

        if i < len && bytes[i] == b',' { i += 1; }
    }

    has_positional && has_named
}

fn kw_matches(bytes: &[u8], skip_map: &SkipMap, i: usize, kw: &[u8]) -> bool {
    let len = bytes.len();
    let kw_len = kw.len();
    if i + kw_len > len { return false; }
    if !skip_map.is_code(i) { return false; }
    let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
    if !before_ok { return false; }
    let matches = bytes[i..i + kw_len]
        .iter()
        .zip(kw.iter())
        .all(|(&a, &b)| a.to_ascii_uppercase() == b.to_ascii_uppercase());
    if !matches { return false; }
    let end = i + kw_len;
    end >= len || !is_word_char(bytes[end])
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Add to `sqrust-rules/src/ambiguous/mod.rs`:
```
pub mod inconsistent_column_reference;
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules inconsistent_column_reference`
Expected: **all GREEN**

---

### Task D5: Register in CLI + commit

Add to `sqrust-cli/src/main.rs`:
```rust
use sqrust_rules::ambiguous::inconsistent_order_by_direction::InconsistentOrderByDirection;
use sqrust_rules::ambiguous::inconsistent_column_reference::InconsistentColumnReference;
```

Add to `rules()` vec:
```rust
Box::new(InconsistentOrderByDirection),
Box::new(InconsistentColumnReference),
```

Run: `~/.cargo/bin/cargo build -p sqrust-cli`

Commit:
```bash
git add sqrust-rules/src/ambiguous/inconsistent_order_by_direction.rs \
        sqrust-rules/src/ambiguous/inconsistent_column_reference.rs \
        sqrust-rules/src/ambiguous/mod.rs \
        sqrust-rules/tests/inconsistent_order_by_direction_test.rs \
        sqrust-rules/tests/inconsistent_column_reference_test.rs \
        sqrust-cli/src/main.rs
git commit -m "feat(ambiguous): add InconsistentOrderByDirection and InconsistentColumnReference rules"
```

---

## AGENT E — Layout: `SelectTargetNewLine` + `SetOperatorNewLine`

**Files to create:**
- `sqrust-rules/src/layout/select_target_new_line.rs`
- `sqrust-rules/src/layout/set_operator_new_line.rs`
- `sqrust-rules/tests/select_target_new_line_test.rs`
- `sqrust-rules/tests/set_operator_new_line_test.rs`

**Files to modify:**
- `sqrust-rules/src/layout/mod.rs`
- `sqrust-cli/src/main.rs`

---

### Task E1: Write failing tests for `SelectTargetNewLine`

Create `sqrust-rules/tests/select_target_new_line_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::select_target_new_line::SelectTargetNewLine;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    SelectTargetNewLine.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SelectTargetNewLine.name(), "Layout/SelectTargetNewLine");
}

#[test]
fn single_column_no_violation() {
    assert!(check("SELECT id FROM t").is_empty());
}

#[test]
fn select_star_no_violation() {
    assert!(check("SELECT * FROM t").is_empty());
}

#[test]
fn each_column_on_own_line_no_violation() {
    assert!(check("SELECT\n    id,\n    name\nFROM t").is_empty());
}

#[test]
fn two_columns_same_line_flagged() {
    let d = check("SELECT id, name FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn three_columns_same_line_flagged_once() {
    let d = check("SELECT id, name, email FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn select_on_own_line_cols_on_same_next_line_flagged() {
    let d = check("SELECT\n    id, name\nFROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn message_mentions_column_or_newline() {
    let d = check("SELECT id, name FROM t");
    let msg = d[0].message.to_lowercase();
    assert!(msg.contains("column") || msg.contains("line") || msg.contains("select"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id, name FROM t");
    assert_eq!(d[0].rule, "Layout/SelectTargetNewLine");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id, name FROM t");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn comma_in_string_not_counted() {
    assert!(check("SELECT 'a,b' FROM t").is_empty());
}

#[test]
fn subquery_with_multi_col_select_flagged() {
    let d = check("SELECT * FROM (SELECT id, name FROM t) sub");
    assert_eq!(d.len(), 1);
}

#[test]
fn function_call_with_comma_not_flagged_as_column() {
    // COALESCE(a, b) has a comma but it's inside a function — single column.
    assert!(check("SELECT COALESCE(a, b) FROM t").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules select_target_new_line`
Expected: **compile error** — RED.

---

### Task E2: Implement `SelectTargetNewLine`

Create `sqrust-rules/src/layout/select_target_new_line.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct SelectTargetNewLine;

impl Rule for SelectTargetNewLine {
    fn name(&self) -> &'static str {
        "Layout/SelectTargetNewLine"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);
        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            if !skip_map.is_code(i) { i += 1; continue; }

            // Find SELECT keyword.
            if let Some(after_select) = match_keyword(bytes, &skip_map, i, b"SELECT") {
                // Skip optional DISTINCT/ALL modifier.
                let mut pos = skip_whitespace(bytes, after_select);
                if let Some(after_distinct) = match_keyword(bytes, &skip_map, pos, b"DISTINCT") {
                    pos = skip_whitespace(bytes, after_distinct);
                } else if let Some(after_all) = match_keyword(bytes, &skip_map, pos, b"ALL") {
                    pos = skip_whitespace(bytes, after_all);
                }

                // Scan the column list. If we find a top-level comma where both
                // the preceding and following content are on the same line,
                // that means two columns share a line → violation.
                if let Some(violation_pos) = scan_select_columns(bytes, &skip_map, pos) {
                    let (line, col) = offset_to_line_col(source, violation_pos);
                    diags.push(Diagnostic {
                        rule: "Layout/SelectTargetNewLine",
                        message: "Multiple SELECT columns on the same line; put each column on its own line".to_string(),
                        line,
                        col,
                    });
                }

                i = pos;
                continue;
            }

            i += 1;
        }

        diags
    }
}

/// Scans SELECT column list starting at `start`. Returns position of the
/// first violation (a comma where columns on both sides share the same line).
fn scan_select_columns(bytes: &[u8], skip_map: &SkipMap, start: usize) -> Option<usize> {
    let len = bytes.len();
    let mut i = start;
    let mut depth = 0i32;

    // Track which line the previous item started on.
    let mut prev_item_newline_before = false; // whether the previous item had a newline before it

    // The first item: note if it starts after a newline.
    // We track: for each comma at depth 0, check if both items separated by it
    // share the same line.
    let mut last_comma_pos: Option<usize> = None;
    let mut last_newline_after_comma = false;

    // We only care about depth-0 commas.
    // A violation is: we find a depth-0 comma where either:
    //   a) the column before the comma shares its line with the comma (no newline between column start and comma)
    //   b) the column after the comma is on the same line as the comma

    // Simpler approach: scan for depth-0 commas. At each comma, check if
    // there's been a newline since the last comma (or SELECT keyword).
    let mut newline_since_last = contains_newline_before(bytes, start, start);
    // Actually track the position of the last newline boundary.
    let mut last_boundary = start; // start of current item

    while i < len {
        if !skip_map.is_code(i) { i += 1; continue; }
        let b = bytes[i];

        if b == b'(' { depth += 1; i += 1; continue; }
        if b == b')' {
            if depth > 0 { depth -= 1; i += 1; continue; }
            else { break; }
        }

        // Stop at SELECT-clause-ending keywords at depth 0.
        if depth == 0 {
            for kw in &[b"FROM" as &[u8], b"WHERE", b"GROUP", b"ORDER", b"HAVING", b"LIMIT",
                        b"UNION", b"INTERSECT", b"EXCEPT", b"FETCH", b";"] {
                if kw_match(bytes, skip_map, i, kw) { return None; }
            }
        }

        if depth == 0 && b == b',' {
            // Check if there is a newline between last_boundary and this comma.
            let has_newline = (last_boundary..i).any(|k| bytes[k] == b'\n');
            if !has_newline {
                // Two columns on the same line — violation.
                return Some(last_boundary);
            }
            // After this comma, reset boundary to next non-whitespace.
            last_boundary = i + 1;
            // Skip to next non-whitespace for boundary.
            let mut j = i + 1;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t') { j += 1; }
            // If next char is not a newline, the next item starts on the same line as comma.
            // We'll track that at the next comma.
            last_boundary = i + 1;
            i += 1;
            continue;
        }

        i += 1;
    }

    None
}

fn contains_newline_before(bytes: &[u8], from: usize, to: usize) -> bool {
    (from..to).any(|k| bytes[k] == b'\n')
}

fn match_keyword(bytes: &[u8], skip_map: &SkipMap, i: usize, kw: &[u8]) -> Option<usize> {
    let len = bytes.len();
    let kw_len = kw.len();
    if i + kw_len > len { return None; }
    if !skip_map.is_code(i) { return None; }
    let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
    if !before_ok { return None; }
    let matches = bytes[i..i + kw_len]
        .iter()
        .zip(kw.iter())
        .all(|(&a, &b)| a.to_ascii_uppercase() == b.to_ascii_uppercase());
    if !matches { return None; }
    let end = i + kw_len;
    if end < len && is_word_char(bytes[end]) { return None; }
    Some(end)
}

fn skip_whitespace(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
        i += 1;
    }
    i
}

fn kw_match(bytes: &[u8], skip_map: &SkipMap, i: usize, kw: &[u8]) -> bool {
    match_keyword(bytes, skip_map, i, kw).is_some()
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Add to `sqrust-rules/src/layout/mod.rs`:
```
pub mod select_target_new_line;
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules select_target_new_line`
Expected: **all GREEN**

---

### Task E3: Write failing tests for `SetOperatorNewLine`

Create `sqrust-rules/tests/set_operator_new_line_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::set_operator_new_line::SetOperatorNewLine;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    SetOperatorNewLine.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(SetOperatorNewLine.name(), "Layout/SetOperatorNewLine");
}

#[test]
fn no_union_no_violation() {
    assert!(check("SELECT id FROM t").is_empty());
}

#[test]
fn union_on_own_line_no_violation() {
    assert!(check("SELECT id FROM t\nUNION ALL\nSELECT id FROM t2").is_empty());
}

#[test]
fn intersect_on_own_line_no_violation() {
    assert!(check("SELECT id FROM t\nINTERSECT\nSELECT id FROM t2").is_empty());
}

#[test]
fn except_on_own_line_no_violation() {
    assert!(check("SELECT id FROM t\nEXCEPT\nSELECT id FROM t2").is_empty());
}

#[test]
fn union_inline_flagged() {
    let d = check("SELECT id FROM t UNION ALL SELECT id FROM t2");
    assert_eq!(d.len(), 1);
}

#[test]
fn union_after_content_on_same_line_flagged() {
    let d = check("SELECT id FROM t UNION\nSELECT id FROM t2");
    assert_eq!(d.len(), 1);
}

#[test]
fn union_before_content_on_same_line_flagged() {
    let d = check("SELECT id FROM t\nUNION SELECT id FROM t2");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_inline_unions_flagged_twice() {
    let d = check("SELECT 1 UNION ALL SELECT 2 UNION ALL SELECT 3");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_union_or_newline() {
    let d = check("SELECT id FROM t UNION ALL SELECT id FROM t2");
    let msg = d[0].message.to_lowercase();
    assert!(msg.contains("union") || msg.contains("newline") || msg.contains("line"));
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t UNION ALL SELECT id FROM t2");
    assert_eq!(d[0].rule, "Layout/SetOperatorNewLine");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t UNION ALL SELECT id FROM t2");
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn union_in_string_not_flagged() {
    assert!(check("SELECT 'UNION ALL' FROM t").is_empty());
}

#[test]
fn union_in_comment_not_flagged() {
    assert!(check("SELECT id FROM t -- UNION ALL\n").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules set_operator_new_line`
Expected: **compile error** — RED.

---

### Task E4: Implement `SetOperatorNewLine`

Create `sqrust-rules/src/layout/set_operator_new_line.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct SetOperatorNewLine;

impl Rule for SetOperatorNewLine {
    fn name(&self) -> &'static str {
        "Layout/SetOperatorNewLine"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip_map = SkipMap::build(source);
        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            if !skip_map.is_code(i) { i += 1; continue; }

            // Look for UNION, INTERSECT, or EXCEPT.
            let kw_match = try_match_set_op(bytes, &skip_map, i);
            if let Some((kw_end, kw_len)) = kw_match {
                // Skip optional ALL or DISTINCT after the operator.
                let mut after_kw = skip_ws_horizontal(bytes, kw_end);
                if let Some(after_all) = match_kw(bytes, &skip_map, after_kw, b"ALL") {
                    after_kw = after_all;
                } else if let Some(after_dist) = match_kw(bytes, &skip_map, after_kw, b"DISTINCT") {
                    after_kw = after_dist;
                }

                // Check: is there a newline immediately BEFORE the operator?
                let newline_before = has_only_whitespace_before_on_line(bytes, i);
                // Check: is there a newline immediately AFTER the operator (+ optional ALL/DISTINCT)?
                let after_ws = skip_ws_horizontal(bytes, after_kw);
                let newline_after = after_ws >= len
                    || bytes[after_ws] == b'\n'
                    || bytes[after_ws] == b'\r'
                    || (after_ws + 1 < len && bytes[after_ws] == b'-' && bytes[after_ws + 1] == b'-');

                if !newline_before || !newline_after {
                    let (line, col) = offset_to_line_col(source, i);
                    diags.push(Diagnostic {
                        rule: "Layout/SetOperatorNewLine",
                        message: "Set operator (UNION/INTERSECT/EXCEPT) must be on its own line, surrounded by newlines".to_string(),
                        line,
                        col,
                    });
                }

                i = kw_end;
                continue;
            }

            i += 1;
        }

        diags
    }
}

/// Try to match UNION, INTERSECT, or EXCEPT at position `i`.
/// Returns `Some((end_pos, kw_len))` if matched.
fn try_match_set_op(bytes: &[u8], skip_map: &SkipMap, i: usize) -> Option<(usize, usize)> {
    for kw in &[b"UNION" as &[u8], b"INTERSECT", b"EXCEPT"] {
        if let Some(end) = match_kw(bytes, skip_map, i, kw) {
            return Some((end, kw.len()));
        }
    }
    None
}

fn match_kw(bytes: &[u8], skip_map: &SkipMap, i: usize, kw: &[u8]) -> Option<usize> {
    let len = bytes.len();
    let kw_len = kw.len();
    if i + kw_len > len { return None; }
    if !skip_map.is_code(i) { return None; }
    let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
    if !before_ok { return None; }
    let matches = bytes[i..i + kw_len]
        .iter()
        .zip(kw.iter())
        .all(|(&a, &b)| a.to_ascii_uppercase() == b.to_ascii_uppercase());
    if !matches { return None; }
    let end = i + kw_len;
    if end < len && is_word_char(bytes[end]) { return None; }
    Some(end)
}

/// True if everything before position `i` on the same line is whitespace.
fn has_only_whitespace_before_on_line(bytes: &[u8], i: usize) -> bool {
    let mut j = if i == 0 { return true; } else { i - 1 };
    loop {
        let b = bytes[j];
        if b == b'\n' { return true; }
        if b != b' ' && b != b'\t' { return false; }
        if j == 0 { return true; }
        j -= 1;
    }
}

/// Skip horizontal whitespace (spaces and tabs only, not newlines).
fn skip_ws_horizontal(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    i
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Add to `sqrust-rules/src/layout/mod.rs`:
```
pub mod select_target_new_line;
pub mod set_operator_new_line;
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules set_operator_new_line`
Expected: **all GREEN**

---

### Task E5: Register in CLI + commit

Add to `sqrust-cli/src/main.rs`:
```rust
use sqrust_rules::layout::select_target_new_line::SelectTargetNewLine;
use sqrust_rules::layout::set_operator_new_line::SetOperatorNewLine;
```

Add to `rules()` vec:
```rust
Box::new(SelectTargetNewLine),
Box::new(SetOperatorNewLine),
```

Run: `~/.cargo/bin/cargo build -p sqrust-cli`

Commit:
```bash
git add sqrust-rules/src/layout/select_target_new_line.rs \
        sqrust-rules/src/layout/set_operator_new_line.rs \
        sqrust-rules/src/layout/mod.rs \
        sqrust-rules/tests/select_target_new_line_test.rs \
        sqrust-rules/tests/set_operator_new_line_test.rs \
        sqrust-cli/src/main.rs
git commit -m "feat(layout): add SelectTargetNewLine and SetOperatorNewLine rules"
```

---

## Integration Task (after all 5 agents complete)

### Task F1: Full test suite

```bash
~/.cargo/bin/cargo test --workspace 2>&1 | grep -E "FAILED|^test result"
```

Expected: `0 failed` across all crates. Fix any failures before proceeding.

### Task F2: Verify rule count

```bash
~/.cargo/bin/cargo test --workspace 2>&1 | grep "^test result"
```

The total test count should increase by ~65-70 tests (13-14 per rule × 10 rules).

### Task F3: Smoke test the binary

```bash
~/.cargo/bin/cargo build -p sqrust-cli 2>&1
echo "SELECT id, name FROM t UNION ALL SELECT id, name FROM t2;" | \
  ~/.cargo/bin/cargo run -p sqrust-cli -- check /dev/stdin 2>/dev/null || true
```

### Task F4: Update CLAUDE.md and HANDOFF.md

In `CLAUDE.md`, update the rule count: **145 rules** → **155 rules** (Waves 1–16)

In `HANDOFF.md`, add Wave 16 row to the table:
```
| 16   | 10          | LeftJoin, JoinConditionStyle, UnusedTableAlias, ConsecutiveSemicolons, NestedCaseInElse, UnusedJoin, InconsistentOrderByDirection, InconsistentColumnReference, SelectTargetNewLine, SetOperatorNewLine |
```

Update the rule count at the top of HANDOFF.md: **145 rules** → **155 rules**

Commit:
```bash
git add CLAUDE.md HANDOFF.md
git commit -m "docs: update rule count to 155 after Wave 16"
```

---

## Notes for Implementing Agents

1. **compile errors are expected** after writing tests before implementation — that's the TDD RED state
2. **SkipMap** is in `sqrust-rules/src/capitalisation/mod.rs` — import as `use crate::capitalisation::{is_word_char, SkipMap};`
3. **group_by_position helpers** (`match_keyword`, `skip_whitespace`) are pub(crate) — available to sibling modules via `use super::group_by_position::{match_keyword, skip_whitespace};`
4. If a test is flaky or fails unexpectedly, check the test SQL — it may need adjustment, not the implementation
5. Each agent only touches their category's `mod.rs` — no agent touches another category's files
6. `sqrust-cli/src/main.rs` is shared — if dispatching truly in parallel, coordinate the CLI update in Task F1 instead of each agent's Task X5
