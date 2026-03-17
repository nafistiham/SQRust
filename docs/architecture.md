# SQRust — Architecture

## Overview

SQRust is a SQL linter written in Rust. It parses SQL files, walks the AST, and applies 300 lint rules in a single parallel pass. The design follows the same core insight as Ruff: do everything in one compiled pass rather than interpreting rules in a slow runtime.

```
sqrust check models/
      │
      ├── Config::load()          Walk up from path to find sqrust.toml
      │
      ├── File discovery          Glob .sql files, apply exclude patterns
      │
      ├── rayon par_iter()        One thread per file
      │    │
      │    ├── FileContext::from_source()   Parse with sqlparser-rs
      │    │
      │    └── for rule in rules            Apply all enabled rules
      │         ├── AST rules    walk Statement tree
      │         └── Text rules   byte-scan with SkipMap
      │
      └── Collect + sort diagnostics, print output
```

---

## Workspace layout

```
sqrust-core/       Rule trait, Diagnostic, FileContext, Config
sqrust-rules/      One file per rule (300 rules across 6 categories)
sqrust-cli/        Binary: argument parsing, file walking, output
```

### sqrust-core

The minimal shared library. Nothing here imports from `sqrust-rules` or `sqrust-cli`.

**`lib.rs`** defines three public types:

```rust
pub struct Diagnostic {
    pub rule: &'static str,
    pub message: String,
    pub line: usize,   // 1-indexed
    pub col: usize,    // 1-indexed
}

pub struct FileContext {
    pub path: PathBuf,
    pub source: String,
    pub statements: Vec<Statement>,   // parsed AST
    pub parse_errors: Vec<String>,
}

pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic>;
    fn fix(&self, _ctx: &FileContext) -> Option<String> { None }
}
```

**`config.rs`** implements `Config::load(path)` — walks up the directory tree from the linted path to find `sqrust.toml`. If no config file is found, all rules are enabled with no exclusions.

### sqrust-rules

Each rule lives in its own file: `src/<category>/<rule_name>.rs`. The file implements a struct and the `Rule` trait on it.

Rule categories: `ambiguous`, `capitalisation`, `convention`, `layout`, `lint`, `structure`.

### sqrust-cli

The `main.rs` binary handles:
- `sqrust check <path>` — lint files
- `sqrust fmt <path>` — auto-fix layout violations
- `sqrust rules` — browse and toggle rules
- `sqrust explain <RuleName>` — show rule description

---

## Two rule types

### AST rules

Walk the `sqlparser-rs` `Statement` tree. These are clean and precise — they can't fire on SQL inside string literals because the parser already stripped those.

Example:

```rust
// Convention/SelectStar — detect SELECT *
fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for stmt in &ctx.statements {
        if let Statement::Query(q) = stmt {
            // walk SELECT items looking for Wildcard
        }
    }
    diags
}
```

### Text-scan rules

Do byte-level scanning of the raw SQL source. Needed for layout rules where whitespace and formatting information is not preserved in the AST.

The key challenge: text patterns must not fire inside string literals or comments. SQRust uses a **`SkipMap`** — a bitset marking every byte that is inside a string literal or comment as non-code. Text-scan rules check `skip_map[offset]` before recording a violation.

Example:

```rust
// Layout/TrailingWhitespace — scan lines for trailing spaces
fn check(&self, ctx: &FileContext) -> Vec<Diagnostic> {
    for (line_no, line) in ctx.lines() {
        if line.ends_with(' ') || line.ends_with('\t') {
            // emit Diagnostic
        }
    }
}
```

---

## Parallelism

File-level parallelism via `rayon`. Each file is processed independently:

```rust
let results: Vec<Vec<Diagnostic>> = files
    .par_iter()
    .map(|path| check_file(path, &rules, &config))
    .collect();
```

Rules within a file are applied sequentially. This keeps the hot path allocation-free: each rule gets a `&FileContext` (read-only) and produces a `Vec<Diagnostic>`.

---

## Configuration

`sqrust.toml` is discovered by walking up from the linted path. Config affects two things:

1. **File exclusion** — `[sqrust] exclude = ["dbt_packages/**"]` — glob patterns applied to each file path before linting
2. **Rule selection** — `[rules] disable = ["Convention/SelectStar"]` — list of rule names to skip

Config is loaded once per `sqrust` invocation and passed as a shared reference to all threads.

The `[rules] select` / `[rules] ignore` (Ruff-style allowlist) fields are reserved for v0.2.0. The parser rejects them with a clear error today (`#[serde(deny_unknown_fields)]`).

---

## Output format

Default: one line per violation, sorted by file path then line number.

```
models/orders.sql:12:5: [Convention/SelectStar] Avoid SELECT *; list columns explicitly
```

JSON output: `sqrust check --format json` — available since v0.1.1.

---

## Rule registration

All 300 rules are instantiated in `sqrust-cli/src/main.rs` as a `Vec<Box<dyn Rule>>`. Adding a new rule requires:

1. Create `sqrust-rules/src/<category>/<rule_name>.rs`
2. `impl Rule for <RuleName>`
3. Add to `mod.rs` in the category
4. Add to the rule list in `main.rs`

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `sqlparser` | SQL parsing — produces the AST |
| `rayon` | File-level parallelism |
| `clap` | CLI argument parsing |
| `toml` + `serde` | `sqrust.toml` config deserialization |
| `glob` | Exclude pattern matching |

---

## Performance profile

On 495 SQL files (jaffle-shop + attribution-playbook + mrr-playbook corpus, duplicated 20×):

- **42 ms** median (300 rules, 4-core MacBook)
- Parse time dominates at this scale — `sqlparser-rs` is the bottleneck, not rule evaluation
- Startup overhead: < 5 ms (no JVM, no Python interpreter)

The 2× gap over `sqruff` (79 ms on the same corpus) is partly explained by the rule count difference (300 vs 62) and partly by the text-scan rules doing more byte-level work.
