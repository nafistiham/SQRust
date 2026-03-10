# Contributing to SQRust

Thanks for your interest. SQRust is a SQL linter written in Rust — contributions of all kinds are welcome.

## Ways to Contribute

- **Report a false positive or false negative** — open an issue with the SQL that triggers it
- **Suggest a new rule** — open an issue describing the anti-pattern and why it matters
- **Fix a bug** — pick up any open issue labeled `bug`
- **Add a rule** — see [Adding a Rule](#adding-a-rule) below

## Development Setup

```bash
git clone https://github.com/nafistiham/SQRust.git
cd SQRust
cargo test --workspace   # should be 0 failures
```

Requires Rust stable (1.70+). No other dependencies.

## Adding a Rule

Each rule lives in one file. The pattern is always the same:

**1. Pick a category:** `convention` / `lint` / `structure` / `ambiguous` / `layout`

**2. Create the rule file:**
```
sqrust-rules/src/<category>/<rule_name>.rs
```

**3. Create the test file first (TDD):**
```
sqrust-rules/tests/<rule_name>_test.rs
```

Write ≥ 13 tests. Run them — they should fail (RED). Then implement until they pass (GREEN).

**4. Register the rule:**

In `sqrust-rules/src/<category>/mod.rs`:
```rust
pub mod rule_name;
```

In `sqrust-cli/src/main.rs`, add a `use` import and a `Box::new(RuleName)` entry in the `rules()` vec.

**5. Open a PR.** The title should follow the pattern:
```
feat(RuleName): add <short description>
```

## Rule Template

```rust
use sqrust_core::{Diagnostic, FileContext, Rule};

pub struct MyRule;

impl Rule for MyRule {
    fn name(&self) -> &'static str {
        "Category/MyRule"
    }

    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
        // AST-based rules: return vec![] on parse errors
        // if !ctx.parse_errors.is_empty() { return vec![]; }
        vec![]
    }
}
```

## Test Template

```rust
use sqrust_core::{FileContext, Rule};
use sqrust_rules::category::my_rule::MyRule;

fn check(sql: &str) -> Vec<sqrust_core::Diagnostic> {
    MyRule.check(&FileContext::from_source(sql, "test.sql"))
}

#[test]
fn rule_name_is_correct() {
    assert_eq!(MyRule.name(), "Category/MyRule");
}

#[test]
fn violation_detected() {
    let d = check("SELECT ... bad SQL ...");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].rule, "Category/MyRule");
}

#[test]
fn no_violation_on_valid_sql() {
    assert!(check("SELECT id FROM t").is_empty());
}
```

## Commit Style

```
feat(RuleName): add rule description
fix(RuleName): short description of fix
test(RuleName): add missing edge case
docs: update rule catalog
```

## Code Review

All PRs get a review. We check:
- ≥ 13 tests, covering both valid and invalid SQL
- No false positives on common SQL patterns
- Position reporting (line/col) is accurate
- No `unwrap()` on external input

## Questions?

Open an issue — no question is too small.
