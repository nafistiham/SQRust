# Wave 17 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add 10 new lint rules across 5 categories (2 per category), bringing the total from 155 → 165.

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
// + other imports

pub struct RuleName;

impl Rule for RuleName {
    fn name(&self) -> &'static str { "Category/RuleName" }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // AST-based rules: if !ctx.parse_errors.is_empty() { return Vec::new(); }
        // text-scan rules: skip_map handles bad input gracefully
        vec![]
    }
}
```

**SkipMap helper** (import as `use crate::capitalisation::{is_word_char, SkipMap};`):
- `SkipMap::build(source)` — marks bytes inside strings/comments as skip
- `skip_map.is_code(i)` — true if byte at `i` is real SQL (not string/comment)
- `is_word_char(byte)` — true for `[a-zA-Z0-9_]`

**offset_to_line_col helper** (copy into each rule file that needs it):
```rust
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

**Position-finding helpers must return `Option<usize>`**, never bare `usize`, to avoid silent (1,1) fallback.

---

## AGENT A — Convention: `ExplicitAlias` + `OrInsteadOfIn`

**Files to create:**
- `sqrust-rules/src/convention/explicit_alias.rs`
- `sqrust-rules/src/convention/or_instead_of_in.rs`
- `sqrust-rules/tests/explicit_alias_test.rs`
- `sqrust-rules/tests/or_instead_of_in_test.rs`

**Files to modify:**
- `sqrust-rules/src/convention/mod.rs` — add two `pub mod` lines
- `sqrust-cli/src/main.rs` — add two `use` lines and two `Box::new(...)` entries

---

### Task A1: Write failing tests for `ExplicitAlias`

Rule: `"Convention/ExplicitAlias"` — Table aliases in FROM/JOIN clauses must use the `AS` keyword.

Create `sqrust-rules/tests/explicit_alias_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::convention::explicit_alias::ExplicitAlias;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ExplicitAlias.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ExplicitAlias.name(), "Convention/ExplicitAlias");
}

#[test]
fn explicit_alias_no_violation() {
    assert!(check("SELECT id FROM t AS alias").is_empty());
}

#[test]
fn no_alias_no_violation() {
    assert!(check("SELECT id FROM t WHERE id = 1").is_empty());
}

#[test]
fn join_with_as_no_violation() {
    assert!(check("SELECT t.id FROM t AS t1 JOIN u AS u1 ON t1.id = u1.t_id").is_empty());
}

#[test]
fn implicit_table_alias_flagged() {
    let d = check("SELECT id FROM orders o WHERE o.id = 1");
    assert_eq!(d.len(), 1);
}

#[test]
fn implicit_join_alias_flagged() {
    let d = check("SELECT t.id FROM t JOIN u u1 ON t.id = u1.t_id");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_implicit_aliases_flagged() {
    let d = check("SELECT a.id FROM accounts a JOIN orders o ON a.id = o.account_id");
    assert_eq!(d.len(), 2);
}

#[test]
fn subquery_with_implicit_alias_flagged() {
    let d = check("SELECT id FROM (SELECT id FROM t) sub");
    assert_eq!(d.len(), 1);
}

#[test]
fn subquery_with_explicit_alias_no_violation() {
    assert!(check("SELECT id FROM (SELECT id FROM t) AS sub").is_empty());
}

#[test]
fn message_mentions_as() {
    let d = check("SELECT id FROM t alias");
    assert_eq!(d.len(), 1);
    assert!(
        d[0].message.to_uppercase().contains("AS"),
        "expected message to mention AS, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t alias");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Convention/ExplicitAlias");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t alias");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn alias_in_string_not_flagged() {
    assert!(check("SELECT 'FROM t alias' FROM t AS real_alias").is_empty());
}

#[test]
fn lateral_join_with_explicit_alias_no_violation() {
    assert!(check("SELECT t.id FROM t JOIN u AS u1 ON t.id = u1.id").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules explicit_alias`
Expected: **compile error** (module not found) — that's the RED state.

---

### Task A2: Implement `ExplicitAlias`

Create `sqrust-rules/src/convention/explicit_alias.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct ExplicitAlias;

impl Rule for ExplicitAlias {
    fn name(&self) -> &'static str {
        "Convention/ExplicitAlias"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();

        // SQL keywords that can follow a table reference (not an alias)
        let non_alias_keywords: &[&[u8]] = &[
            b"WHERE", b"ON", b"SET", b"GROUP", b"ORDER", b"HAVING", b"LIMIT",
            b"UNION", b"INTERSECT", b"EXCEPT", b"JOIN", b"INNER", b"LEFT",
            b"RIGHT", b"FULL", b"OUTER", b"CROSS", b"LATERAL", b"USING",
            b"FETCH", b"OFFSET", b"FOR", b"INTO", b"VALUES", b"RETURNING",
        ];

        let mut i = 0;
        while i < len {
            if !skip.is_code(i) {
                i += 1;
                continue;
            }

            // Look for FROM or JOIN keyword
            if !is_word_char(bytes[i]) || (i > 0 && is_word_char(bytes[i - 1])) {
                i += 1;
                continue;
            }

            // Read word
            let ws = i;
            let mut we = i;
            while we < len && is_word_char(bytes[we]) {
                we += 1;
            }
            let word = &bytes[ws..we];

            let is_from = word.eq_ignore_ascii_case(b"FROM");
            let is_join = word.len() >= 4 && {
                let suffix = &word[word.len() - 4..];
                suffix.eq_ignore_ascii_case(b"JOIN")
            };

            if is_from || is_join {
                // Skip whitespace after FROM/JOIN
                let mut j = we;
                while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                    j += 1;
                }
                if j >= len || !skip.is_code(j) {
                    i = we;
                    continue;
                }

                // Read table reference: either a word (table name) or a '(' (subquery)
                let table_end;
                if bytes[j] == b'(' {
                    // Skip over the parenthesized subquery
                    let mut depth = 0usize;
                    let mut k = j;
                    while k < len {
                        if skip.is_code(k) {
                            if bytes[k] == b'(' { depth += 1; }
                            else if bytes[k] == b')' {
                                depth -= 1;
                                if depth == 0 { k += 1; break; }
                            }
                        }
                        k += 1;
                    }
                    table_end = k;
                } else {
                    // Read table name (possibly schema.name)
                    let mut k = j;
                    while k < len && (is_word_char(bytes[k]) || bytes[k] == b'.') {
                        k += 1;
                    }
                    table_end = k;
                }

                if table_end == 0 || table_end >= len {
                    i = we;
                    continue;
                }

                // Skip whitespace after table/subquery
                let mut k = table_end;
                while k < len && (bytes[k] == b' ' || bytes[k] == b'\t') {
                    k += 1;
                }

                // Check what's next
                if k >= len || !skip.is_code(k) || bytes[k] == b'\n' || bytes[k] == b'\r' || bytes[k] == b',' || bytes[k] == b')' || bytes[k] == b';' {
                    i = we;
                    continue;
                }

                // Check for AS keyword
                if is_word_char(bytes[k]) {
                    let as_start = k;
                    let mut ae = k;
                    while ae < len && is_word_char(bytes[ae]) {
                        ae += 1;
                    }
                    let next_word = &bytes[as_start..ae];

                    if next_word.eq_ignore_ascii_case(b"AS") {
                        // Good — explicit alias with AS
                        i = ae;
                        continue;
                    }

                    // Check if it's a non-alias keyword
                    let is_non_alias = non_alias_keywords.iter().any(|kw| next_word.eq_ignore_ascii_case(kw));
                    if is_non_alias {
                        i = we;
                        continue;
                    }

                    // It's an implicit alias — flag it
                    let (line, col) = offset_to_line_col(source, as_start);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: format!(
                            "Table alias '{}' should use the AS keyword",
                            String::from_utf8_lossy(next_word)
                        ),
                        line,
                        col,
                    });
                    i = ae;
                    continue;
                }
            }

            i = we;
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

Run: `~/.cargo/bin/cargo test -p sqrust-rules explicit_alias`
Expected: **all GREEN** — 14 tests passing.

---

### Task A3: Write failing tests for `OrInsteadOfIn`

Rule: `"Convention/OrInsteadOfIn"` — Three or more `col = x OR col = y OR col = z` on the same column should use `IN()`.

Create `sqrust-rules/tests/or_instead_of_in_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::convention::or_instead_of_in::OrInsteadOfIn;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    OrInsteadOfIn.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(OrInsteadOfIn.name(), "Convention/OrInsteadOfIn");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn two_or_conditions_same_col_no_violation() {
    assert!(check("SELECT id FROM t WHERE status = 'a' OR status = 'b'").is_empty());
}

#[test]
fn three_or_conditions_same_col_flagged() {
    let d = check("SELECT id FROM t WHERE status = 'a' OR status = 'b' OR status = 'c'");
    assert_eq!(d.len(), 1);
}

#[test]
fn four_or_conditions_same_col_flagged_once() {
    let d = check("SELECT id FROM t WHERE s = 1 OR s = 2 OR s = 3 OR s = 4");
    assert_eq!(d.len(), 1);
}

#[test]
fn three_different_cols_no_violation() {
    assert!(check("SELECT id FROM t WHERE a = 1 OR b = 2 OR c = 3").is_empty());
}

#[test]
fn two_same_one_diff_no_violation() {
    assert!(check("SELECT id FROM t WHERE a = 1 OR b = 2 OR a = 3").is_empty());
}

#[test]
fn in_clause_used_correctly_no_violation() {
    assert!(check("SELECT id FROM t WHERE status IN ('a', 'b', 'c')").is_empty());
}

#[test]
fn three_or_in_having_flagged() {
    let d = check("SELECT dept FROM t GROUP BY dept HAVING dept = 'a' OR dept = 'b' OR dept = 'c'");
    assert_eq!(d.len(), 1);
}

#[test]
fn message_mentions_in() {
    let d = check("SELECT id FROM t WHERE x = 1 OR x = 2 OR x = 3");
    assert_eq!(d.len(), 1);
    assert!(
        d[0].message.to_uppercase().contains("IN"),
        "expected message to mention IN, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t WHERE x = 1 OR x = 2 OR x = 3");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Convention/OrInsteadOfIn");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t WHERE x = 1 OR x = 2 OR x = 3");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn qualified_col_three_or_flagged() {
    let d = check("SELECT id FROM t WHERE t.status = 'a' OR t.status = 'b' OR t.status = 'c'");
    assert_eq!(d.len(), 1);
}

#[test]
fn mixed_qualified_unqualified_three_no_violation() {
    // Different column name forms — conservative: don't flag if mixed qualified/unqualified
    // (treating t.status and status as different for simplicity)
    assert!(check("SELECT id FROM t WHERE t.status = 'a' OR status = 'b' OR t.status = 'c'").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules or_instead_of_in`
Expected: **compile error** — RED.

---

### Task A4: Implement `OrInsteadOfIn`

Create `sqrust-rules/src/convention/or_instead_of_in.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, With};
use std::collections::HashMap;

pub struct OrInsteadOfIn;

impl Rule for OrInsteadOfIn {
    fn name(&self) -> &'static str {
        "Convention/OrInsteadOfIn"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            check_stmt(stmt, &ctx.source, self.name(), &mut diags);
        }
        diags
    }
}

fn check_stmt(stmt: &Statement, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match stmt {
        Statement::Query(q) => check_query(q, src, rule, diags),
        _ => {}
    }
}

fn check_query(q: &Query, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if let Some(With { cte_tables, .. }) = &q.with {
        for cte in cte_tables {
            check_query(&cte.query, src, rule, diags);
        }
    }
    check_set_expr(&q.body, src, rule, diags);
}

fn check_set_expr(body: &SetExpr, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match body {
        SetExpr::Select(s) => check_select(s, src, rule, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, src, rule, diags);
            check_set_expr(right, src, rule, diags);
        }
        SetExpr::Query(q) => check_query(q, src, rule, diags),
        _ => {}
    }
}

fn check_select(sel: &Select, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if let Some(expr) = &sel.selection {
        check_expr_for_or_chains(expr, src, rule, diags);
    }
    if let Some(expr) = &sel.having {
        check_expr_for_or_chains(expr, src, rule, diags);
    }
}

fn check_expr_for_or_chains(expr: &Expr, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    // Collect the full OR chain at this level
    let mut equalities: Vec<(String, usize)> = Vec::new(); // (col_name, source_offset)
    collect_or_equalities(expr, &mut equalities, src);

    if equalities.len() >= 2 {
        // Group by column name
        let mut counts: HashMap<&str, Vec<usize>> = HashMap::new();
        for (col, off) in &equalities {
            counts.entry(col.as_str()).or_default().push(*off);
        }
        for (col, offsets) in &counts {
            if offsets.len() >= 3 {
                let off = offsets[0];
                let (line, col_pos) = offset_to_line_col(src, off);
                diags.push(Diagnostic {
                    rule,
                    message: format!(
                        "Column '{}' appears in {} OR equality conditions; use IN() instead",
                        col, offsets.len()
                    ),
                    line,
                    col: col_pos,
                });
            }
        }
        return; // processed this level
    }

    // Recurse into sub-expressions (non-OR operators)
    match expr {
        Expr::BinaryOp { left, right, .. } => {
            // If not all OR, recurse into branches
            if equalities.is_empty() {
                check_expr_for_or_chains(left, src, rule, diags);
                check_expr_for_or_chains(right, src, rule, diags);
            }
        }
        Expr::Nested(inner) => check_expr_for_or_chains(inner, src, rule, diags),
        Expr::Not(inner) => check_expr_for_or_chains(inner, src, rule, diags),
        _ => {}
    }
}

/// Recursively collects (column_name, offset) for each `col = literal` in an OR chain.
/// Only collects from BinaryOp(Or) chains; stops at non-Or operators.
fn collect_or_equalities(expr: &Expr, out: &mut Vec<(String, usize)>, src: &str) {
    use sqlparser::ast::BinaryOperator;
    match expr {
        Expr::BinaryOp { left, op: BinaryOperator::Or, right } => {
            collect_or_equalities(left, out, src);
            collect_or_equalities(right, out, src);
        }
        Expr::BinaryOp { left, op: BinaryOperator::Eq, right } => {
            // Check for col = literal pattern
            let col_name = expr_to_col_name(left)
                .or_else(|| expr_to_col_name(right));
            if let Some(name) = col_name {
                // Find position of the column name in source
                if let Some(off) = find_word_in_source(src, &name, 0) {
                    out.push((name, off));
                }
            }
        }
        Expr::Nested(inner) => collect_or_equalities(inner, out, src),
        _ => {}
    }
}

fn expr_to_col_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Identifier(i) => Some(i.value.to_lowercase()),
        Expr::CompoundIdentifier(parts) => {
            // Use the full dotted path for qualified refs
            Some(parts.iter().map(|p| p.value.to_lowercase()).collect::<Vec<_>>().join("."))
        }
        _ => None,
    }
}

fn find_word_in_source(src: &str, word: &str, start: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let wbytes = word.as_bytes();
    let wlen = wbytes.len();
    if wlen == 0 || start + wlen > bytes.len() {
        return None;
    }
    let mut i = start;
    while i + wlen <= bytes.len() {
        if bytes[i..i + wlen].eq_ignore_ascii_case(wbytes) {
            let before_ok = i == 0 || !is_word_char_plain(bytes[i - 1]);
            let after_ok = i + wlen >= bytes.len() || !is_word_char_plain(bytes[i + wlen]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_word_char_plain(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules or_instead_of_in`
Expected: **all GREEN**.

---

### Task A5: Register in `mod.rs`

Add to `sqrust-rules/src/convention/mod.rs`:
```
pub mod explicit_alias;
pub mod or_instead_of_in;
```

---

## AGENT B — Lint: `ColumnAliasInWhere` + `DuplicateJoin`

**Files to create:**
- `sqrust-rules/src/lint/column_alias_in_where.rs`
- `sqrust-rules/src/lint/duplicate_join.rs`
- `sqrust-rules/tests/column_alias_in_where_test.rs`
- `sqrust-rules/tests/duplicate_join_test.rs`

**Files to modify:**
- `sqrust-rules/src/lint/mod.rs` — add two `pub mod` lines
- `sqrust-cli/src/main.rs` — add two `use` lines and two `Box::new(...)` entries

---

### Task B1: Write failing tests for `ColumnAliasInWhere`

Rule: `"Lint/ColumnAliasInWhere"` — A column alias defined in SELECT is referenced in the WHERE clause (invalid ANSI SQL; WHERE is evaluated before SELECT aliases are defined).

Create `sqrust-rules/tests/column_alias_in_where_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::lint::column_alias_in_where::ColumnAliasInWhere;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ColumnAliasInWhere.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ColumnAliasInWhere.name(), "Lint/ColumnAliasInWhere");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn no_alias_in_where_no_violation() {
    assert!(check("SELECT id, name FROM t WHERE id > 1").is_empty());
}

#[test]
fn alias_not_in_where_no_violation() {
    assert!(check("SELECT a + b AS total FROM t WHERE a > 1").is_empty());
}

#[test]
fn alias_in_where_flagged() {
    let d = check("SELECT a + b AS total FROM t WHERE total > 100");
    assert_eq!(d.len(), 1);
}

#[test]
fn alias_in_where_case_insensitive() {
    let d = check("SELECT a + b AS Total FROM t WHERE total > 100");
    assert_eq!(d.len(), 1);
}

#[test]
fn two_aliases_in_where_two_violations() {
    let d = check("SELECT a AS x, b AS y FROM t WHERE x > 1 AND y > 2");
    assert_eq!(d.len(), 2);
}

#[test]
fn alias_in_order_by_no_violation() {
    // ORDER BY alias is allowed in some dialects; we only flag WHERE
    assert!(check("SELECT a + b AS total FROM t ORDER BY total").is_empty());
}

#[test]
fn alias_in_having_no_violation() {
    // HAVING can reference aggregate aliases in some dialects — don't flag
    assert!(check("SELECT dept, COUNT(*) AS cnt FROM t GROUP BY dept HAVING cnt > 5").is_empty());
}

#[test]
fn alias_same_as_column_name_flagged() {
    // Even if the alias matches a real column name, we flag conservatively
    let d = check("SELECT id AS id FROM t WHERE id > 1");
    // Actually id is also a real column name — this may produce a false positive.
    // The test documents the conservative behavior.
    let _ = d; // Don't assert count — behavior is conservative
}

#[test]
fn message_mentions_alias() {
    let d = check("SELECT a + b AS total FROM t WHERE total > 0");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("alias") || msg.contains("where") || msg.contains("total"),
        "expected message to mention alias/where/column name, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT a + b AS total FROM t WHERE total > 0");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Lint/ColumnAliasInWhere");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT a + b AS total FROM t WHERE total > 0");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn subquery_alias_in_outer_where_not_flagged() {
    // The alias is defined in a subquery's SELECT; outer WHERE has its own column refs
    assert!(check("SELECT * FROM (SELECT a + b AS total FROM t) sub WHERE sub.total > 0").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules column_alias_in_where`
Expected: **compile error** — RED.

---

### Task B2: Implement `ColumnAliasInWhere`

Create `sqrust-rules/src/lint/column_alias_in_where.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Expr, Query, Select, SelectItem, SetExpr, Statement, With};
use std::collections::HashSet;

pub struct ColumnAliasInWhere;

impl Rule for ColumnAliasInWhere {
    fn name(&self) -> &'static str {
        "Lint/ColumnAliasInWhere"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            check_stmt(stmt, &ctx.source, "Lint/ColumnAliasInWhere", &mut diags);
        }
        diags
    }
}

fn check_stmt(stmt: &Statement, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if let Statement::Query(q) = stmt {
        check_query(q, src, rule, diags);
    }
}

fn check_query(q: &Query, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if let Some(With { cte_tables, .. }) = &q.with {
        for cte in cte_tables {
            check_query(&cte.query, src, rule, diags);
        }
    }
    check_set_expr(&q.body, src, rule, diags);
}

fn check_set_expr(body: &SetExpr, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match body {
        SetExpr::Select(s) => check_select(s, src, rule, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, src, rule, diags);
            check_set_expr(right, src, rule, diags);
        }
        SetExpr::Query(q) => check_query(q, src, rule, diags),
        _ => {}
    }
}

fn check_select(sel: &Select, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    // Collect SELECT aliases
    let mut aliases: HashSet<String> = HashSet::new();
    for item in &sel.projection {
        if let SelectItem::ExprWithAlias { alias, .. } = item {
            aliases.insert(alias.value.to_lowercase());
        }
    }

    if aliases.is_empty() {
        return;
    }

    // Walk WHERE for identifiers matching aliases
    if let Some(where_expr) = &sel.selection {
        find_alias_refs(where_expr, &aliases, src, rule, diags);
    }
}

fn find_alias_refs(
    expr: &Expr,
    aliases: &HashSet<String>,
    src: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::Identifier(ident) => {
            let lower = ident.value.to_lowercase();
            if aliases.contains(&lower) {
                if let Some(off) = find_word_in_source(src, &ident.value, 0) {
                    let (line, col) = offset_to_line_col(src, off);
                    diags.push(Diagnostic {
                        rule,
                        message: format!(
                            "Column alias '{}' is used in WHERE clause; aliases are not available in WHERE (evaluated before SELECT)",
                            ident.value
                        ),
                        line,
                        col,
                    });
                }
            }
        }
        Expr::BinaryOp { left, right, .. } => {
            find_alias_refs(left, aliases, src, rule, diags);
            find_alias_refs(right, aliases, src, rule, diags);
        }
        Expr::UnaryOp { expr, .. } | Expr::Nested(expr) | Expr::Not(expr) => {
            find_alias_refs(expr, aliases, src, rule, diags);
        }
        Expr::Between { expr, low, high, .. } => {
            find_alias_refs(expr, aliases, src, rule, diags);
            find_alias_refs(low, aliases, src, rule, diags);
            find_alias_refs(high, aliases, src, rule, diags);
        }
        Expr::InList { expr, list, .. } => {
            find_alias_refs(expr, aliases, src, rule, diags);
            for e in list {
                find_alias_refs(e, aliases, src, rule, diags);
            }
        }
        Expr::IsNull(e) | Expr::IsNotNull(e) => find_alias_refs(e, aliases, src, rule, diags),
        Expr::Like { expr, pattern, .. } | Expr::ILike { expr, pattern, .. } => {
            find_alias_refs(expr, aliases, src, rule, diags);
            find_alias_refs(pattern, aliases, src, rule, diags);
        }
        _ => {}
    }
}

fn find_word_in_source(src: &str, word: &str, start: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let wbytes = word.as_bytes();
    let wlen = wbytes.len();
    if wlen == 0 { return None; }
    let mut i = start;
    while i + wlen <= bytes.len() {
        if bytes[i..i + wlen].eq_ignore_ascii_case(wbytes) {
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + wlen >= bytes.len() || !is_word_char(bytes[i + wlen]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules column_alias_in_where`
Expected: **all GREEN**.

---

### Task B3: Write failing tests for `DuplicateJoin`

Rule: `"Lint/DuplicateJoin"` — The same table appears more than once in the FROM/JOIN clause of a single SELECT.

Create `sqrust-rules/tests/duplicate_join_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::lint::duplicate_join::DuplicateJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    DuplicateJoin.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(DuplicateJoin.name(), "Lint/DuplicateJoin");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn no_joins_no_violation() {
    assert!(check("SELECT id FROM t WHERE id > 1").is_empty());
}

#[test]
fn two_different_tables_no_violation() {
    assert!(check("SELECT t.id FROM t JOIN u ON t.id = u.t_id").is_empty());
}

#[test]
fn same_table_twice_flagged() {
    let d = check("SELECT a.id FROM orders a JOIN orders b ON a.parent = b.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn same_table_three_times_flagged() {
    let d = check("SELECT a.id FROM t a JOIN t b ON a.p = b.id JOIN t c ON b.p = c.id");
    assert_eq!(d.len(), 1);
}

#[test]
fn main_table_and_join_same_flagged() {
    let d = check("SELECT t.id FROM t JOIN t AS t2 ON t.id = t2.parent_id");
    assert_eq!(d.len(), 1);
}

#[test]
fn schema_qualified_same_table_flagged() {
    let d = check("SELECT a.id FROM schema1.orders a JOIN schema1.orders b ON a.id = b.ref");
    assert_eq!(d.len(), 1);
}

#[test]
fn different_schemas_same_name_no_violation() {
    // schema1.orders and schema2.orders are different tables
    assert!(check("SELECT a.id FROM schema1.orders a JOIN schema2.orders b ON a.id = b.ref").is_empty());
}

#[test]
fn message_mentions_duplicate() {
    let d = check("SELECT a.id FROM t a JOIN t b ON a.id = b.parent");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("duplicate") || msg.contains("joined") || msg.contains("twice") || msg.contains("more than once"),
        "expected message to mention duplicate join, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT a.id FROM t a JOIN t b ON a.id = b.p");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Lint/DuplicateJoin");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT a.id FROM t a JOIN t b ON a.id = b.p");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn subquery_own_joins_checked_independently() {
    // Subquery's own duplicate join should be flagged
    let d = check("SELECT * FROM (SELECT a.id FROM t a JOIN t b ON a.id = b.p) sub");
    assert_eq!(d.len(), 1);
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules duplicate_join`
Expected: **compile error** — RED.

---

### Task B4: Implement `DuplicateJoin`

Create `sqrust-rules/src/lint/duplicate_join.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Query, Select, SetExpr, Statement, TableFactor, TableWithJoins, With,
};
use std::collections::HashMap;

pub struct DuplicateJoin;

impl Rule for DuplicateJoin {
    fn name(&self) -> &'static str {
        "Lint/DuplicateJoin"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            check_stmt(stmt, &ctx.source, "Lint/DuplicateJoin", &mut diags);
        }
        diags
    }
}

fn check_stmt(stmt: &Statement, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if let Statement::Query(q) = stmt {
        check_query(q, src, rule, diags);
    }
}

fn check_query(q: &Query, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if let Some(With { cte_tables, .. }) = &q.with {
        for cte in cte_tables {
            check_query(&cte.query, src, rule, diags);
        }
    }
    check_set_expr(&q.body, src, rule, diags);
}

fn check_set_expr(body: &SetExpr, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match body {
        SetExpr::Select(s) => check_select(s, src, rule, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, src, rule, diags);
            check_set_expr(right, src, rule, diags);
        }
        SetExpr::Query(q) => check_query(q, src, rule, diags),
        _ => {}
    }
}

fn check_select(sel: &Select, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    for twj in &sel.from {
        check_table_with_joins(twj, src, rule, diags);
    }
}

fn check_table_with_joins(twj: &TableWithJoins, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    // Collect all table names (lowercased full name) with first occurrence offset
    let mut seen: HashMap<String, usize> = HashMap::new();

    // Main table
    if let Some((name, off)) = table_factor_name(&twj.relation, src) {
        seen.insert(name, off);
    }

    // Recurse into subqueries in main table
    check_factor_subqueries(&twj.relation, src, rule, diags);

    // JOINs
    let mut flagged = false;
    for join in &twj.joins {
        check_factor_subqueries(&join.relation, src, rule, diags);
        if let Some((name, off)) = table_factor_name(&join.relation, src) {
            if seen.contains_key(&name) && !flagged {
                let (line, col) = offset_to_line_col(src, off);
                diags.push(Diagnostic {
                    rule,
                    message: format!(
                        "Table '{}' is joined more than once in the same FROM clause",
                        name
                    ),
                    line,
                    col,
                });
                flagged = true;
            } else {
                seen.insert(name, off);
            }
        }
    }
}

fn table_factor_name(tf: &TableFactor, src: &str) -> Option<(String, usize)> {
    match tf {
        TableFactor::Table { name, .. } => {
            let full_name = name.0.iter()
                .map(|i| i.value.to_lowercase())
                .collect::<Vec<_>>()
                .join(".");
            let last = name.0.last()?.value.clone();
            let off = find_word_in_source(src, &last, 0)?;
            Some((full_name, off))
        }
        _ => None,
    }
}

fn check_factor_subqueries(tf: &TableFactor, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if let TableFactor::Derived { subquery, .. } = tf {
        check_query(subquery, src, rule, diags);
    }
}

fn find_word_in_source(src: &str, word: &str, start: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let wbytes = word.as_bytes();
    let wlen = wbytes.len();
    if wlen == 0 { return None; }
    let mut i = start;
    while i + wlen <= bytes.len() {
        if bytes[i..i + wlen].eq_ignore_ascii_case(wbytes) {
            let before_ok = i == 0 || !is_wc(bytes[i - 1]);
            let after_ok = i + wlen >= bytes.len() || !is_wc(bytes[i + wlen]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_wc(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules duplicate_join`
Expected: **all GREEN**.

---

### Task B5: Register in `mod.rs`

Add to `sqrust-rules/src/lint/mod.rs`:
```
pub mod column_alias_in_where;
pub mod duplicate_join;
```

---

## AGENT C — Structure: `WildcardInUnion` + `UnqualifiedColumnInJoin`

**Files to create:**
- `sqrust-rules/src/structure/wildcard_in_union.rs`
- `sqrust-rules/src/structure/unqualified_column_in_join.rs`
- `sqrust-rules/tests/wildcard_in_union_test.rs`
- `sqrust-rules/tests/unqualified_column_in_join_test.rs`

**Files to modify:**
- `sqrust-rules/src/structure/mod.rs` — add two `pub mod` lines
- `sqrust-cli/src/main.rs` — add two `use` lines and two `Box::new(...)` entries

---

### Task C1: Write failing tests for `WildcardInUnion`

Rule: `"Structure/WildcardInUnion"` — `SELECT *` used in any branch of a UNION/INTERSECT/EXCEPT. When table schemas change, wildcard-UNION silently maps wrong columns.

Create `sqrust-rules/tests/wildcard_in_union_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::structure::wildcard_in_union::WildcardInUnion;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    WildcardInUnion.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(WildcardInUnion.name(), "Structure/WildcardInUnion");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn no_union_no_violation() {
    assert!(check("SELECT * FROM t").is_empty());
}

#[test]
fn union_explicit_columns_no_violation() {
    assert!(check("SELECT id, name FROM t UNION ALL SELECT id, name FROM u").is_empty());
}

#[test]
fn wildcard_in_first_union_branch_flagged() {
    let d = check("SELECT * FROM t UNION ALL SELECT id, name FROM u");
    assert_eq!(d.len(), 1);
}

#[test]
fn wildcard_in_second_union_branch_flagged() {
    let d = check("SELECT id, name FROM t UNION ALL SELECT * FROM u");
    assert_eq!(d.len(), 1);
}

#[test]
fn wildcard_in_both_union_branches_flagged() {
    let d = check("SELECT * FROM t UNION ALL SELECT * FROM u");
    assert_eq!(d.len(), 2);
}

#[test]
fn intersect_with_wildcard_flagged() {
    let d = check("SELECT * FROM t INTERSECT SELECT id FROM u");
    assert_eq!(d.len(), 1);
}

#[test]
fn except_with_wildcard_flagged() {
    let d = check("SELECT * FROM t EXCEPT SELECT id FROM u");
    assert_eq!(d.len(), 1);
}

#[test]
fn message_mentions_wildcard_or_union() {
    let d = check("SELECT * FROM t UNION ALL SELECT id FROM u");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("wildcard") || msg.contains("*") || msg.contains("union") || msg.contains("select *"),
        "expected message to mention wildcard or union, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT * FROM t UNION ALL SELECT id FROM u");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Structure/WildcardInUnion");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT * FROM t UNION ALL SELECT id FROM u");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn qualified_wildcard_in_union_flagged() {
    // t.* is also a wildcard
    let d = check("SELECT t.* FROM t UNION ALL SELECT id FROM u");
    assert_eq!(d.len(), 1);
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules wildcard_in_union`
Expected: **compile error** — RED.

---

### Task C2: Implement `WildcardInUnion`

Create `sqrust-rules/src/structure/wildcard_in_union.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{Query, SelectItem, SetExpr, Statement, With};

pub struct WildcardInUnion;

impl Rule for WildcardInUnion {
    fn name(&self) -> &'static str {
        "Structure/WildcardInUnion"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, &ctx.source, self.name(), &mut diags, false);
            }
        }
        diags
    }
}

/// `in_union` is true when this SetExpr is a branch of a UNION/INTERSECT/EXCEPT.
fn check_query(q: &Query, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>, in_union: bool) {
    if let Some(With { cte_tables, .. }) = &q.with {
        for cte in cte_tables {
            check_query(&cte.query, src, rule, diags, false);
        }
    }
    check_set_expr(&q.body, src, rule, diags, in_union);
}

fn check_set_expr(body: &SetExpr, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>, in_union: bool) {
    match body {
        SetExpr::Select(sel) => {
            if in_union {
                for item in &sel.projection {
                    match item {
                        SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => {
                            if let Some(off) = find_select_star(src) {
                                let (line, col) = offset_to_line_col(src, off);
                                diags.push(Diagnostic {
                                    rule,
                                    message: "SELECT * in a UNION/INTERSECT/EXCEPT branch is fragile; list columns explicitly".to_string(),
                                    line,
                                    col,
                                });
                            }
                            break; // one diagnostic per SELECT branch
                        }
                        _ => {}
                    }
                }
            }
        }
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, src, rule, diags, true);
            check_set_expr(right, src, rule, diags, true);
        }
        SetExpr::Query(q) => check_query(q, src, rule, diags, in_union),
        _ => {}
    }
}

fn find_select_star(src: &str) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'*' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

**Note on `find_select_star`:** The simple scan for `*` might not find the right position when both branches use wildcards. The implementer should find the `*` corresponding to the current SELECT. One approach: since `check_set_expr` processes left branch first then right, use a `start_offset` parameter. The agent should refine this to accurately track positions per branch, perhaps by scanning from a given byte offset.

Run: `~/.cargo/bin/cargo test -p sqrust-rules wildcard_in_union`
Expected: **all GREEN**.

---

### Task C3: Write failing tests for `UnqualifiedColumnInJoin`

Rule: `"Structure/UnqualifiedColumnInJoin"` — In a query with explicit JOINs, column references should be qualified with a table name or alias (`t.col`, not just `col`). Unqualified columns are ambiguous when multiple tables are present.

Create `sqrust-rules/tests/unqualified_column_in_join_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::structure::unqualified_column_in_join::UnqualifiedColumnInJoin;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    UnqualifiedColumnInJoin.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(UnqualifiedColumnInJoin.name(), "Structure/UnqualifiedColumnInJoin");
}

#[test]
fn parse_error_returns_no_violations() {
    assert!(check("SELECT FROM FROM WHERE").is_empty());
}

#[test]
fn single_table_no_join_no_violation() {
    assert!(check("SELECT id, name FROM t WHERE id > 1").is_empty());
}

#[test]
fn all_qualified_with_join_no_violation() {
    assert!(check("SELECT t.id, u.name FROM t JOIN u ON t.id = u.t_id WHERE t.id > 1").is_empty());
}

#[test]
fn unqualified_select_col_with_join_flagged() {
    let d = check("SELECT id, name FROM t JOIN u ON t.id = u.t_id");
    assert!(!d.is_empty());
}

#[test]
fn unqualified_where_col_with_join_flagged() {
    let d = check("SELECT t.id FROM t JOIN u ON t.id = u.t_id WHERE id > 5");
    assert!(!d.is_empty());
}

#[test]
fn wildcard_select_no_violation() {
    // SELECT * is not a column ref — don't flag
    assert!(check("SELECT * FROM t JOIN u ON t.id = u.t_id").is_empty());
}

#[test]
fn count_star_no_violation() {
    // COUNT(*) — the * inside is not a column ref
    assert!(check("SELECT COUNT(*) FROM t JOIN u ON t.id = u.t_id").is_empty());
}

#[test]
fn message_mentions_qualify() {
    let d = check("SELECT id FROM t JOIN u ON t.id = u.t_id");
    assert!(!d.is_empty());
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("qualif") || msg.contains("table") || msg.contains("alias"),
        "expected message to mention qualifying columns, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t JOIN u ON t.id = u.t_id");
    assert!(!d.is_empty());
    assert_eq!(d[0].rule, "Structure/UnqualifiedColumnInJoin");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t JOIN u ON t.id = u.t_id");
    assert!(!d.is_empty());
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn function_arg_unqualified_flagged() {
    let d = check("SELECT UPPER(name) FROM t JOIN u ON t.id = u.t_id");
    assert!(!d.is_empty());
}

#[test]
fn on_clause_not_flagged() {
    // ON clause qualifications are expected and not flagged by this rule
    // (they must be qualified for the JOIN to make sense)
    // This test just ensures the rule doesn't double-count ON cols
    let d = check("SELECT t.id FROM t JOIN u ON t.id = u.t_id");
    assert!(d.is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules unqualified_column_in_join`
Expected: **compile error** — RED.

---

### Task C4: Implement `UnqualifiedColumnInJoin`

Create `sqrust-rules/src/structure/unqualified_column_in_join.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use sqlparser::ast::{
    Expr, FunctionArg, FunctionArgExpr, Query, Select, SelectItem, SetExpr,
    Statement, TableWithJoins, With,
};

pub struct UnqualifiedColumnInJoin;

impl Rule for UnqualifiedColumnInJoin {
    fn name(&self) -> &'static str {
        "Structure/UnqualifiedColumnInJoin"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        if !ctx.parse_errors.is_empty() {
            return Vec::new();
        }
        let mut diags = Vec::new();
        for stmt in &ctx.statements {
            if let Statement::Query(q) = stmt {
                check_query(q, &ctx.source, self.name(), &mut diags);
            }
        }
        diags
    }
}

fn check_query(q: &Query, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    if let Some(With { cte_tables, .. }) = &q.with {
        for cte in cte_tables {
            check_query(&cte.query, src, rule, diags);
        }
    }
    check_set_expr(&q.body, src, rule, diags);
}

fn check_set_expr(body: &SetExpr, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match body {
        SetExpr::Select(s) => check_select(s, src, rule, diags),
        SetExpr::SetOperation { left, right, .. } => {
            check_set_expr(left, src, rule, diags);
            check_set_expr(right, src, rule, diags);
        }
        SetExpr::Query(q) => check_query(q, src, rule, diags),
        _ => {}
    }
}

fn check_select(sel: &Select, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    // Only flag when there are JOINs
    let has_joins = sel.from.iter().any(|twj| !twj.joins.is_empty());
    if !has_joins {
        return;
    }

    // Check SELECT projections
    for item in &sel.projection {
        match item {
            SelectItem::UnnamedExpr(e) | SelectItem::ExprWithAlias { expr: e, .. } => {
                find_unqualified(e, src, rule, diags);
            }
            SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(_, _) => {}
        }
    }

    // Check WHERE
    if let Some(w) = &sel.selection {
        find_unqualified(w, src, rule, diags);
    }

    // Check HAVING
    if let Some(h) = &sel.having {
        find_unqualified(h, src, rule, diags);
    }

    // Check GROUP BY
    for g in &sel.group_by {
        find_unqualified(g, src, rule, diags);
    }
}

fn find_unqualified(expr: &Expr, src: &str, rule: &'static str, diags: &mut Vec<Diagnostic>) {
    match expr {
        Expr::Identifier(i) => {
            // Unqualified — flag it
            if let Some(off) = find_word_in_source(src, &i.value, 0) {
                let (line, col) = offset_to_line_col(src, off);
                diags.push(Diagnostic {
                    rule,
                    message: format!(
                        "Column '{}' is not qualified with a table name or alias; in a JOIN query, all columns should be table-qualified",
                        i.value
                    ),
                    line,
                    col,
                });
            }
        }
        Expr::CompoundIdentifier(_) => {} // Qualified — ok
        Expr::BinaryOp { left, right, .. } => {
            find_unqualified(left, src, rule, diags);
            find_unqualified(right, src, rule, diags);
        }
        Expr::UnaryOp { expr, .. } | Expr::Nested(expr) | Expr::Not(expr) => {
            find_unqualified(expr, src, rule, diags);
        }
        Expr::Function(f) => {
            for arg in &f.args {
                match arg {
                    FunctionArg::Named { arg, .. } | FunctionArg::Unnamed(arg) => {
                        match arg {
                            FunctionArgExpr::Expr(e) => find_unqualified(e, src, rule, diags),
                            FunctionArgExpr::Wildcard | FunctionArgExpr::QualifiedWildcard(_) => {}
                        }
                    }
                }
            }
        }
        Expr::IsNull(e) | Expr::IsNotNull(e) => find_unqualified(e, src, rule, diags),
        Expr::Between { expr, low, high, .. } => {
            find_unqualified(expr, src, rule, diags);
            find_unqualified(low, src, rule, diags);
            find_unqualified(high, src, rule, diags);
        }
        Expr::InList { expr, list, .. } => {
            find_unqualified(expr, src, rule, diags);
            for e in list {
                find_unqualified(e, src, rule, diags);
            }
        }
        Expr::Case { operand, conditions, results, else_result } => {
            if let Some(e) = operand { find_unqualified(e, src, rule, diags); }
            for (c, r) in conditions.iter().zip(results.iter()) {
                find_unqualified(c, src, rule, diags);
                find_unqualified(r, src, rule, diags);
            }
            if let Some(e) = else_result { find_unqualified(e, src, rule, diags); }
        }
        _ => {}
    }
}

fn find_word_in_source(src: &str, word: &str, start: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let wbytes = word.as_bytes();
    let wlen = wbytes.len();
    if wlen == 0 { return None; }
    let mut i = start;
    while i + wlen <= bytes.len() {
        if bytes[i..i + wlen].eq_ignore_ascii_case(wbytes) {
            let before_ok = i == 0 || !is_wc(bytes[i - 1]);
            let after_ok = i + wlen >= bytes.len() || !is_wc(bytes[i + wlen]);
            if before_ok && after_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn is_wc(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules unqualified_column_in_join`
Expected: **all GREEN**.

---

### Task C5: Register in `mod.rs`

Add to `sqrust-rules/src/structure/mod.rs`:
```
pub mod wildcard_in_union;
pub mod unqualified_column_in_join;
```

---

## AGENT D — Ambiguous: `FloatingPointComparison` + `AmbiguousDateFormat`

**Files to create:**
- `sqrust-rules/src/ambiguous/floating_point_comparison.rs`
- `sqrust-rules/src/ambiguous/ambiguous_date_format.rs`
- `sqrust-rules/tests/floating_point_comparison_test.rs`
- `sqrust-rules/tests/ambiguous_date_format_test.rs`

**Files to modify:**
- `sqrust-rules/src/ambiguous/mod.rs` — add two `pub mod` lines
- `sqrust-cli/src/main.rs` — add two `use` lines and two `Box::new(...)` entries

---

### Task D1: Write failing tests for `FloatingPointComparison`

Rule: `"Ambiguous/FloatingPointComparison"` — Exact `=` or `!=`/`<>` comparison with a floating-point literal. Float arithmetic is imprecise; exact equality is almost always wrong.

Create `sqrust-rules/tests/floating_point_comparison_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::floating_point_comparison::FloatingPointComparison;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    FloatingPointComparison.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(FloatingPointComparison.name(), "Ambiguous/FloatingPointComparison");
}

#[test]
fn integer_comparison_no_violation() {
    assert!(check("SELECT id FROM t WHERE price = 10").is_empty());
}

#[test]
fn string_comparison_no_violation() {
    assert!(check("SELECT id FROM t WHERE name = 'hello'").is_empty());
}

#[test]
fn greater_than_float_no_violation() {
    // > and < are fine — not exact equality
    assert!(check("SELECT id FROM t WHERE price > 9.99").is_empty());
}

#[test]
fn eq_float_flagged() {
    let d = check("SELECT id FROM t WHERE price = 9.99");
    assert_eq!(d.len(), 1);
}

#[test]
fn neq_float_flagged() {
    let d = check("SELECT id FROM t WHERE ratio != 0.5");
    assert_eq!(d.len(), 1);
}

#[test]
fn diamond_neq_float_flagged() {
    let d = check("SELECT id FROM t WHERE rate <> 1.5");
    assert_eq!(d.len(), 1);
}

#[test]
fn float_in_string_not_flagged() {
    assert!(check("SELECT id FROM t WHERE name = '9.99'").is_empty());
}

#[test]
fn float_in_comment_not_flagged() {
    assert!(check("SELECT id FROM t -- where price = 9.99\nWHERE id > 1").is_empty());
}

#[test]
fn two_float_comparisons_flagged() {
    let d = check("SELECT id FROM t WHERE price = 9.99 AND ratio != 0.5");
    assert_eq!(d.len(), 2);
}

#[test]
fn message_mentions_float_or_precision() {
    let d = check("SELECT id FROM t WHERE price = 9.99");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("float") || msg.contains("precision") || msg.contains("exact") || msg.contains("decimal"),
        "expected message to mention float/precision/exact, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t WHERE price = 9.99");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Ambiguous/FloatingPointComparison");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t WHERE price = 9.99");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn zero_point_zero_flagged() {
    let d = check("SELECT id FROM t WHERE ratio = 0.0");
    assert_eq!(d.len(), 1);
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules floating_point_comparison`
Expected: **compile error** — RED.

---

### Task D2: Implement `FloatingPointComparison`

Create `sqrust-rules/src/ambiguous/floating_point_comparison.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct FloatingPointComparison;

impl Rule for FloatingPointComparison {
    fn name(&self) -> &'static str {
        "Ambiguous/FloatingPointComparison"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            if !skip.is_code(i) {
                i += 1;
                continue;
            }

            // Look for = != <> operators
            let (is_eq, op_len) = if bytes[i] == b'=' && (i == 0 || bytes[i - 1] != b'!' && bytes[i - 1] != b'<' && bytes[i - 1] != b'>') {
                (true, 1)
            } else if i + 1 < len && bytes[i] == b'!' && bytes[i + 1] == b'=' {
                (true, 2)
            } else if i + 1 < len && bytes[i] == b'<' && bytes[i + 1] == b'>' {
                (true, 2)
            } else {
                (false, 1)
            };

            if !is_eq {
                i += 1;
                continue;
            }

            let op_start = i;
            i += op_len;

            // Skip whitespace after operator
            while i < len && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
                i += 1;
            }

            // Check if what follows is a float literal: optional sign, digits, '.', digits
            let float_start = i;
            // Skip optional sign
            if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
                i += 1;
            }
            // Must have at least one digit
            let digit_start = i;
            while i < len && bytes[i].is_ascii_digit() {
                i += 1;
            }
            // Must have a '.'
            if i < len && bytes[i] == b'.' && i > digit_start {
                i += 1;
                // Must have at least one digit after '.'
                let frac_start = i;
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                if i > frac_start {
                    // Make sure it's not followed by more word chars (like 'e10' making it scientific)
                    let followed_by_word = i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_');
                    if !followed_by_word {
                        // Confirmed float literal
                        let (line, col) = offset_to_line_col(source, op_start);
                        diags.push(Diagnostic {
                            rule: self.name(),
                            message: format!(
                                "Exact equality comparison with floating-point literal at col {}; floating-point values are imprecise — consider using a range check or ROUND()",
                                col
                            ),
                            line,
                            col,
                        });
                        continue;
                    }
                }
            }
            // Not a float — reset i to after operator
            i = float_start;
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

Run: `~/.cargo/bin/cargo test -p sqrust-rules floating_point_comparison`
Expected: **all GREEN**.

---

### Task D3: Write failing tests for `AmbiguousDateFormat`

Rule: `"Ambiguous/AmbiguousDateFormat"` — String literal in slash-separated date format (`'12/01/2023'`) is locale-dependent (MM/DD vs DD/MM). Use ISO 8601 (`'2023-12-01'`) instead.

Create `sqrust-rules/tests/ambiguous_date_format_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::ambiguous::ambiguous_date_format::AmbiguousDateFormat;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    AmbiguousDateFormat.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(AmbiguousDateFormat.name(), "Ambiguous/AmbiguousDateFormat");
}

#[test]
fn iso_date_no_violation() {
    assert!(check("SELECT id FROM t WHERE dt = '2023-12-01'").is_empty());
}

#[test]
fn non_date_string_no_violation() {
    assert!(check("SELECT id FROM t WHERE name = 'hello'").is_empty());
}

#[test]
fn slash_date_mm_dd_yyyy_flagged() {
    let d = check("SELECT id FROM t WHERE dt = '12/01/2023'");
    assert_eq!(d.len(), 1);
}

#[test]
fn slash_date_d_m_yy_flagged() {
    let d = check("SELECT id FROM t WHERE dt = '1/5/24'");
    assert_eq!(d.len(), 1);
}

#[test]
fn slash_date_dd_mm_yyyy_flagged() {
    let d = check("SELECT id FROM t WHERE dt >= '01/12/2023'");
    assert_eq!(d.len(), 1);
}

#[test]
fn year_first_slash_no_violation() {
    // '2023/12/01' — year is first, unambiguous
    assert!(check("SELECT id FROM t WHERE dt = '2023/12/01'").is_empty());
}

#[test]
fn two_slash_dates_flagged() {
    let d = check("SELECT id FROM t WHERE dt BETWEEN '01/01/2023' AND '12/31/2023'");
    assert_eq!(d.len(), 2);
}

#[test]
fn slash_date_in_comment_not_flagged() {
    assert!(check("SELECT id FROM t -- where dt = '12/01/2023'\nWHERE id > 1").is_empty());
}

#[test]
fn message_mentions_iso_or_format() {
    let d = check("SELECT id FROM t WHERE dt = '12/01/2023'");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("iso") || msg.contains("format") || msg.contains("locale") || msg.contains("ambiguous"),
        "expected message to mention ISO/format/locale, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT id FROM t WHERE dt = '12/01/2023'");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Ambiguous/AmbiguousDateFormat");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT id FROM t WHERE dt = '12/01/2023'");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn url_like_string_no_violation() {
    // Not a date pattern (three slashes, not two)
    assert!(check("SELECT id FROM t WHERE url = 'http://example.com/path'").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules ambiguous_date_format`
Expected: **compile error** — RED.

---

### Task D4: Implement `AmbiguousDateFormat`

Create `sqrust-rules/src/ambiguous/ambiguous_date_format.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::SkipMap;

pub struct AmbiguousDateFormat;

impl Rule for AmbiguousDateFormat {
    fn name(&self) -> &'static str {
        "Ambiguous/AmbiguousDateFormat"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        // Build a skip map for comments only — we want to scan string content
        // so we need a different approach: manually find quoted strings.
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            // Only process single-quote string starts that are in code context
            if !skip.is_code(i) {
                i += 1;
                continue;
            }
            if bytes[i] != b'\'' {
                i += 1;
                continue;
            }

            // Found a single-quote string start — collect the content
            let str_start = i;
            i += 1;
            let content_start = i;
            // Find end of string
            while i < len {
                if bytes[i] == b'\'' {
                    if i + 1 < len && bytes[i + 1] == b'\'' {
                        i += 2; // escaped quote
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            let content = &bytes[content_start..i];
            if i < len { i += 1; } // skip closing quote

            // Check if content matches slash date pattern: N+/N+/N+ where first part < 32 (not a year)
            if let Some(_) = is_ambiguous_slash_date(content) {
                let (line, col) = offset_to_line_col(source, str_start);
                diags.push(Diagnostic {
                    rule: self.name(),
                    message: "Date literal uses slash-separated format which is locale-dependent (MM/DD vs DD/MM); use ISO 8601 format ('YYYY-MM-DD') instead".to_string(),
                    line,
                    col,
                });
            }
        }

        diags
    }
}

/// Returns Some(()) if `s` looks like an ambiguous slash-separated date.
/// Pattern: 1-2 digits / 1-2 digits / 2-4 digits, where the first segment < 32 (not a year).
fn is_ambiguous_slash_date(s: &[u8]) -> Option<()> {
    // Trim whitespace
    let s = trim_bytes(s);
    if s.len() < 5 { return None; }

    // Parse first segment
    let (seg1, rest) = read_digits(s)?;
    if seg1.is_empty() || seg1.len() > 2 { return None; }
    let n1: u32 = std::str::from_utf8(seg1).ok()?.parse().ok()?;
    if n1 > 31 { return None; } // year-first format — not ambiguous

    // Must have slash
    if rest.is_empty() || rest[0] != b'/' { return None; }
    let rest = &rest[1..];

    // Parse second segment
    let (seg2, rest) = read_digits(rest)?;
    if seg2.is_empty() || seg2.len() > 2 { return None; }

    // Must have slash
    if rest.is_empty() || rest[0] != b'/' { return None; }
    let rest = &rest[1..];

    // Parse third segment (year: 2 or 4 digits)
    let (seg3, rest) = read_digits(rest)?;
    if seg3.len() < 2 || seg3.len() > 4 { return None; }

    // Must be end of string (allow time component after space)
    if !rest.is_empty() && rest[0] != b' ' && rest[0] != b'T' {
        return None;
    }

    Some(())
}

fn read_digits(s: &[u8]) -> Option<(&[u8], &[u8])> {
    let end = s.iter().position(|&b| !b.is_ascii_digit()).unwrap_or(s.len());
    Some((&s[..end], &s[end..]))
}

fn trim_bytes(s: &[u8]) -> &[u8] {
    let start = s.iter().position(|&b| b != b' ' && b != b'\t').unwrap_or(0);
    let end = s.iter().rposition(|&b| b != b' ' && b != b'\t').map(|i| i + 1).unwrap_or(0);
    if start >= end { &[] } else { &s[start..end] }
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules ambiguous_date_format`
Expected: **all GREEN**.

---

### Task D5: Register in `mod.rs`

Add to `sqrust-rules/src/ambiguous/mod.rs`:
```
pub mod floating_point_comparison;
pub mod ambiguous_date_format;
```

---

## AGENT E — Layout: `ArithmeticOperatorPadding` + `BlankLineAfterCte`

**Files to create:**
- `sqrust-rules/src/layout/arithmetic_operator_padding.rs`
- `sqrust-rules/src/layout/blank_line_after_cte.rs`
- `sqrust-rules/tests/arithmetic_operator_padding_test.rs`
- `sqrust-rules/tests/blank_line_after_cte_test.rs`

**Files to modify:**
- `sqrust-rules/src/layout/mod.rs` — add two `pub mod` lines
- `sqrust-cli/src/main.rs` — add two `use` lines and two `Box::new(...)` entries

---

### Task E1: Write failing tests for `ArithmeticOperatorPadding`

Rule: `"Layout/ArithmeticOperatorPadding"` — Arithmetic operators (`+`, `-`, `*`, `/`, `%`) must have a space on both sides.

Create `sqrust-rules/tests/arithmetic_operator_padding_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::arithmetic_operator_padding::ArithmeticOperatorPadding;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    ArithmeticOperatorPadding.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(ArithmeticOperatorPadding.name(), "Layout/ArithmeticOperatorPadding");
}

#[test]
fn spaced_plus_no_violation() {
    assert!(check("SELECT a + b FROM t").is_empty());
}

#[test]
fn spaced_minus_no_violation() {
    assert!(check("SELECT a - b FROM t").is_empty());
}

#[test]
fn select_star_no_violation() {
    assert!(check("SELECT * FROM t").is_empty());
}

#[test]
fn count_star_no_violation() {
    assert!(check("SELECT COUNT(*) FROM t").is_empty());
}

#[test]
fn unspaced_plus_flagged() {
    let d = check("SELECT a+b FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn unspaced_minus_flagged() {
    let d = check("SELECT a-b FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn unspaced_multiply_flagged() {
    let d = check("SELECT price*1.1 FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn unspaced_divide_flagged() {
    let d = check("SELECT total/count FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn unspaced_modulo_flagged() {
    let d = check("SELECT id%2 FROM t");
    assert_eq!(d.len(), 1);
}

#[test]
fn operator_in_string_not_flagged() {
    assert!(check("SELECT 'a+b' FROM t").is_empty());
}

#[test]
fn line_comment_operator_not_flagged() {
    assert!(check("SELECT id FROM t -- a+b\nWHERE id > 1").is_empty());
}

#[test]
fn message_mentions_space_or_padding() {
    let d = check("SELECT a+b FROM t");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("space") || msg.contains("pad") || msg.contains("operator"),
        "expected message to mention space/padding/operator, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("SELECT a+b FROM t");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Layout/ArithmeticOperatorPadding");
}

#[test]
fn line_col_nonzero() {
    let d = check("SELECT a+b FROM t");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules arithmetic_operator_padding`
Expected: **compile error** — RED.

---

### Task E2: Implement `ArithmeticOperatorPadding`

Create `sqrust-rules/src/layout/arithmetic_operator_padding.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct ArithmeticOperatorPadding;

impl Rule for ArithmeticOperatorPadding {
    fn name(&self) -> &'static str {
        "Layout/ArithmeticOperatorPadding"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;
        let mut paren_depth: i32 = 0;

        while i < len {
            if !skip.is_code(i) {
                i += 1;
                continue;
            }

            // Track parenthesis depth for context
            if bytes[i] == b'(' {
                paren_depth += 1;
                i += 1;
                continue;
            }
            if bytes[i] == b')' {
                if paren_depth > 0 { paren_depth -= 1; }
                i += 1;
                continue;
            }

            // Check for arithmetic operators: + - * / %
            let op = bytes[i];
            if op == b'+' || op == b'-' || op == b'*' || op == b'/' || op == b'%' {
                // Exclusions:
                // -- (line comment start): skip
                if op == b'-' && i + 1 < len && bytes[i + 1] == b'-' {
                    i += 2;
                    while i < len && bytes[i] != b'\n' { i += 1; }
                    continue;
                }
                // /* (block comment start)
                if op == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                    i += 1;
                    continue;
                }
                // * inside parentheses preceded by ( → COUNT(*), SELECT(*) style
                if op == b'*' {
                    let prev_non_ws = prev_non_whitespace(bytes, i);
                    let next_non_ws = next_non_whitespace(bytes, i, len);
                    if prev_non_ws == Some(b'(') || next_non_ws == Some(b')') {
                        i += 1;
                        continue;
                    }
                }
                // Unary +/- after (, =, >, <, !, ,, operators at start of expression
                if op == b'+' || op == b'-' {
                    let prev = prev_non_whitespace(bytes, i);
                    match prev {
                        None | Some(b'(') | Some(b'=') | Some(b'>') | Some(b'<') | Some(b'!') | Some(b',') => {
                            i += 1;
                            continue;
                        }
                        _ => {}
                    }
                }
                // ** exponentiation — treat as two-char operator
                if op == b'*' && i + 1 < len && bytes[i + 1] == b'*' {
                    // skip both
                    i += 2;
                    continue;
                }

                // Check padding: need space (or newline) before AND after
                let space_before = i == 0 || is_space(bytes[i - 1]);
                let space_after = i + 1 >= len || is_space(bytes[i + 1]);

                if !space_before || !space_after {
                    let (line, col) = offset_to_line_col(source, i);
                    diags.push(Diagnostic {
                        rule: self.name(),
                        message: format!(
                            "Arithmetic operator '{}' must be padded with spaces on both sides",
                            bytes[i] as char
                        ),
                        line,
                        col,
                    });
                }
            }

            i += 1;
        }

        diags
    }
}

fn is_space(b: u8) -> bool {
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r'
}

fn prev_non_whitespace(bytes: &[u8], pos: usize) -> Option<u8> {
    if pos == 0 { return None; }
    let mut j = pos - 1;
    loop {
        if !is_space(bytes[j]) {
            return Some(bytes[j]);
        }
        if j == 0 { return None; }
        j -= 1;
    }
}

fn next_non_whitespace(bytes: &[u8], pos: usize, len: usize) -> Option<u8> {
    if pos + 1 >= len { return None; }
    let mut j = pos + 1;
    while j < len {
        if !is_space(bytes[j]) {
            return Some(bytes[j]);
        }
        j += 1;
    }
    None
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules arithmetic_operator_padding`
Expected: **all GREEN**.

---

### Task E3: Write failing tests for `BlankLineAfterCte`

Rule: `"Layout/BlankLineAfterCte"` — Consecutive CTE definitions must be separated by a blank line.

Create `sqrust-rules/tests/blank_line_after_cte_test.rs`:

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::layout::blank_line_after_cte::BlankLineAfterCte;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    BlankLineAfterCte.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(BlankLineAfterCte.name(), "Layout/BlankLineAfterCte");
}

#[test]
fn single_cte_no_violation() {
    assert!(check("WITH a AS (\n    SELECT 1\n)\nSELECT * FROM a").is_empty());
}

#[test]
fn no_cte_no_violation() {
    assert!(check("SELECT id FROM t WHERE id > 1").is_empty());
}

#[test]
fn two_ctes_with_blank_line_no_violation() {
    assert!(check("WITH a AS (\n    SELECT 1\n),\n\nb AS (\n    SELECT 2\n)\nSELECT * FROM a JOIN b ON 1=1").is_empty());
}

#[test]
fn two_ctes_no_blank_line_flagged() {
    let d = check("WITH a AS (\n    SELECT 1\n),\nb AS (\n    SELECT 2\n)\nSELECT * FROM a JOIN b ON 1=1");
    assert_eq!(d.len(), 1);
}

#[test]
fn three_ctes_two_missing_blank_lines_flagged() {
    let sql = "WITH a AS (\n    SELECT 1\n),\nb AS (\n    SELECT 2\n),\nc AS (\n    SELECT 3\n)\nSELECT * FROM a";
    let d = check(sql);
    assert_eq!(d.len(), 2);
}

#[test]
fn three_ctes_first_has_blank_second_missing_flagged() {
    let sql = "WITH a AS (\n    SELECT 1\n),\n\nb AS (\n    SELECT 2\n),\nc AS (\n    SELECT 3\n)\nSELECT * FROM a";
    let d = check(sql);
    assert_eq!(d.len(), 1);
}

#[test]
fn inline_ctes_flagged() {
    let d = check("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a JOIN b ON 1=1");
    assert_eq!(d.len(), 1);
}

#[test]
fn message_mentions_blank_line_or_cte() {
    let d = check("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a");
    assert_eq!(d.len(), 1);
    let msg = d[0].message.to_lowercase();
    assert!(
        msg.contains("blank") || msg.contains("line") || msg.contains("cte"),
        "expected message to mention blank line or CTE, got: {}",
        d[0].message
    );
}

#[test]
fn rule_name_in_diagnostic() {
    let d = check("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Layout/BlankLineAfterCte");
}

#[test]
fn line_col_nonzero() {
    let d = check("WITH a AS (SELECT 1), b AS (SELECT 2) SELECT * FROM a");
    assert_eq!(d.len(), 1);
    assert!(d[0].line >= 1);
    assert!(d[0].col >= 1);
}

#[test]
fn cte_with_nested_parens_no_false_flag() {
    // CTE body has nested parens — should not confuse the depth tracker
    let sql = "WITH a AS (\n    SELECT COALESCE(x, 1) FROM t\n),\n\nb AS (\n    SELECT 2\n)\nSELECT * FROM a";
    assert!(check(sql).is_empty());
}

#[test]
fn single_inline_cte_no_violation() {
    assert!(check("WITH a AS (SELECT 1) SELECT * FROM a").is_empty());
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules blank_line_after_cte`
Expected: **compile error** — RED.

---

### Task E4: Implement `BlankLineAfterCte`

Create `sqrust-rules/src/layout/blank_line_after_cte.rs`:

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};
use crate::capitalisation::{is_word_char, SkipMap};

pub struct BlankLineAfterCte;

impl Rule for BlankLineAfterCte {
    fn name(&self) -> &'static str {
        "Layout/BlankLineAfterCte"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        let source = &ctx.source;
        let bytes = source.as_bytes();
        let len = bytes.len();
        let skip = SkipMap::build(source);

        let mut diags = Vec::new();
        let mut i = 0;

        while i < len {
            if !skip.is_code(i) {
                i += 1;
                continue;
            }

            // Look for WITH keyword at word boundary
            if !is_word_char(bytes[i]) || (i > 0 && is_word_char(bytes[i - 1])) {
                i += 1;
                continue;
            }
            let ws = i;
            let mut we = i;
            while we < len && is_word_char(bytes[we]) { we += 1; }
            let word = &bytes[ws..we];

            if !word.eq_ignore_ascii_case(b"WITH") {
                i = we;
                continue;
            }

            // Found WITH — now scan CTE definitions
            i = we;
            scan_ctes(bytes, len, &skip, &mut i, source, self.name(), &mut diags);
        }

        diags
    }
}

/// After consuming WITH, scan consecutive CTE definitions separated by commas.
/// Flag each comma that is NOT preceded (with only whitespace between) by a blank line.
fn scan_ctes(
    bytes: &[u8],
    len: usize,
    skip: &crate::capitalisation::SkipMap,
    pos: &mut usize,
    source: &str,
    rule: &'static str,
    diags: &mut Vec<Diagnostic>,
) {
    loop {
        // Skip to `AS` keyword and then opening paren
        skip_to_cte_body(bytes, len, skip, pos);
        if *pos >= len { break; }

        // Now we should be at the opening `(` of the CTE body
        if bytes[*pos] != b'(' { break; }

        // Track depth to find closing `)`
        let mut depth = 0usize;
        while *pos < len {
            if skip.is_code(*pos) {
                if bytes[*pos] == b'(' { depth += 1; }
                else if bytes[*pos] == b')' {
                    depth -= 1;
                    if depth == 0 {
                        *pos += 1;
                        break;
                    }
                }
            }
            *pos += 1;
        }

        // Now after the closing `)`.
        // Check if the next code character is `,` (another CTE follows).
        let gap_start = *pos;
        while *pos < len && (bytes[*pos] == b' ' || bytes[*pos] == b'\t' || bytes[*pos] == b'\n' || bytes[*pos] == b'\r') {
            *pos += 1;
        }

        if *pos >= len || bytes[*pos] != b',' {
            break; // No more CTEs (next is main SELECT)
        }

        // Check if there's a blank line in the gap (gap_start..comma)
        let gap = &bytes[gap_start..*pos];
        let has_blank_line = has_double_newline(gap);

        if !has_blank_line {
            let (line, col) = offset_to_line_col(source, *pos);
            diags.push(Diagnostic {
                rule,
                message: "CTE definitions should be separated by a blank line for readability".to_string(),
                line,
                col,
            });
        }

        *pos += 1; // skip comma — continue to next CTE
    }
}

/// Skip forward until we find the opening `(` of a CTE body (past the CTE name and AS keyword).
fn skip_to_cte_body(bytes: &[u8], len: usize, skip: &crate::capitalisation::SkipMap, pos: &mut usize) {
    while *pos < len {
        if !skip.is_code(*pos) { *pos += 1; continue; }
        if bytes[*pos] == b'(' { return; }
        *pos += 1;
    }
}

fn has_double_newline(bytes: &[u8]) -> bool {
    let mut newlines = 0u32;
    for &b in bytes {
        if b == b'\n' {
            newlines += 1;
            if newlines >= 2 { return true; }
        } else if b != b'\r' && b != b' ' && b != b'\t' {
            newlines = 0; // reset on non-whitespace
        }
    }
    false
}

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.chars().filter(|&c| c == '\n').count() + 1;
    let col = before.rfind('\n').map(|p| offset - p - 1).unwrap_or(offset) + 1;
    (line, col)
}
```

Run: `~/.cargo/bin/cargo test -p sqrust-rules blank_line_after_cte`
Expected: **all GREEN**.

---

### Task E5: Register in `mod.rs`

Add to `sqrust-rules/src/layout/mod.rs`:
```
pub mod arithmetic_operator_padding;
pub mod blank_line_after_cte;
```

---

## CLI Registration (all agents contribute)

After all 5 agents complete their category, update `sqrust-cli/src/main.rs`.

Add these `use` lines (after the Wave 16 imports, before `use std::path::PathBuf`):

```rust
// Wave 17
use sqrust_rules::convention::explicit_alias::ExplicitAlias;
use sqrust_rules::convention::or_instead_of_in::OrInsteadOfIn;
use sqrust_rules::lint::column_alias_in_where::ColumnAliasInWhere;
use sqrust_rules::lint::duplicate_join::DuplicateJoin;
use sqrust_rules::structure::wildcard_in_union::WildcardInUnion;
use sqrust_rules::structure::unqualified_column_in_join::UnqualifiedColumnInJoin;
use sqrust_rules::ambiguous::floating_point_comparison::FloatingPointComparison;
use sqrust_rules::ambiguous::ambiguous_date_format::AmbiguousDateFormat;
use sqrust_rules::layout::arithmetic_operator_padding::ArithmeticOperatorPadding;
use sqrust_rules::layout::blank_line_after_cte::BlankLineAfterCte;
```

Add these entries to the `rules()` function (after the `// Wave 16` block, before the closing `]`):

```rust
        // Wave 17
        Box::new(ExplicitAlias),
        Box::new(OrInsteadOfIn),
        Box::new(ColumnAliasInWhere),
        Box::new(DuplicateJoin),
        Box::new(WildcardInUnion),
        Box::new(UnqualifiedColumnInJoin),
        Box::new(FloatingPointComparison),
        Box::new(AmbiguousDateFormat),
        Box::new(ArithmeticOperatorPadding),
        Box::new(BlankLineAfterCte),
```

---

## Integration Check

After all agents complete:

```bash
~/.cargo/bin/cargo test --workspace 2>&1 | grep -E "FAILED|^test result"
```

Expected: `0 failed` across all crates. Fix any failures before proceeding.

---

## Docs Update

After integration passes, update `CLAUDE.md` and `HANDOFF.md`:
- Rule count: 155 → 165
- Add Wave 17 row to the wave table in HANDOFF.md:
  ```
  | 17   | 10          | ExplicitAlias, OrInsteadOfIn, ColumnAliasInWhere, DuplicateJoin, WildcardInUnion, UnqualifiedColumnInJoin, FloatingPointComparison, AmbiguousDateFormat, ArithmeticOperatorPadding, BlankLineAfterCte |
  ```
- Update Quick Summary from 155 → 165

---

## Notes for Agents

1. **Compile errors are expected** after writing tests before implementation — that's the TDD RED state.
2. **Run tests for one rule at a time** to catch issues early.
3. **If a test is flaky or difficult**, check the test SQL — sqlparser-rs must be able to parse it.
4. **Position-finding helpers must return `Option<usize>`** — never bare `usize`.
5. **AST-based rules**: always return early with `Vec::new()` when `!ctx.parse_errors.is_empty()`.
6. **WildcardInUnion position tracking**: the `find_select_star` helper scans for `*` from offset 0 — this will find the first `*` in the query. For accurate per-branch positions, refine by passing a `start_offset` based on which branch you're in. The tests use `assert_eq!(d.len(), N)` to verify count; position accuracy only needs `>= 1`.
7. **UnqualifiedColumnInJoin**: the `find_unqualified` function walks the AST. Be aware that `Identifier` nodes may appear in many places — function names, keyword arguments, etc. The implementation above only flags identifiers that appear as column expressions (not function names themselves). Test carefully with complex queries.
8. **ArithmeticOperatorPadding**: the `-` sign has many uses (unary, negative literal, subtraction). The exclusion logic above handles the most common cases. If tests fail due to false positives on unary minus in `WHERE col > -1`, add `Some(b'>')` and `Some(b'<')` to the unary exclusions.
