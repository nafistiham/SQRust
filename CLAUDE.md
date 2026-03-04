# SQRust — CLAUDE.md

## What This Is

A SQL linter and formatter written in Rust. Targets the dbt / data engineering market as a fast replacement for sqlfluff. The Ruff-for-SQL play.

**Codename:** SQRust (permanent name TBD)

## Tech Stack

- **Language:** Rust (stable toolchain, MSRV TBD)
- **SQL Parser:** `sqlparser-rs` — multi-dialect SQL parser crate
- **Parallelism:** `rayon` — file-level parallel processing
- **Config:** `toml` crate — `sqrust.toml` in project root
- **CLI:** `clap` — subcommands: `check`, `fmt`, `explain`
- **Workspace:** Cargo workspace
  - `sqrust-core` — Rule trait, AST walker, types, file walker
  - `sqrust-rules` — all lint/format rules
  - `sqrust-cli` — binary entry point

## Workspace Layout

```
sqrust-core/      Rule trait, Diagnostic, FileContext, walker
sqrust-rules/     one file per rule, e.g. src/layout/trailing_comma.rs
sqrust-cli/       main.rs, CLI parsing, output formatting
docs/             public-facing docs
internals/        GITIGNORED — competition research, planning, decisions
```

## Branching & Merging

```
main ← production ← develop ← feature/*
```

- **Rebase only** — no merge commits, no squash
- PRs target `develop`
- `gh pr merge --rebase` is the standard merge command
- After merging to develop, fast-forward production and main

## Commit Convention

- Format: `type(Scope): message` — e.g. `fix(TrailingComma): skip CTEs`
- Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`
- Scope = rule name or module name
- No AI/Claude mentions in commit messages
- No `Co-Authored-By` trailers

## Rule Naming

Rules follow RuboCop / sqlfluff convention: `Category/RuleName`

- `Layout/TrailingComma`
- `Style/KeywordCase`
- `Lint/UnreferencedCTE`

Each rule lives in `sqrust-rules/src/<category>/<rule_name>.rs`.

## Rule Trait (initial design — may evolve)

```rust
pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;
    fn category(&self) -> &'static str;
    fn default_enabled(&self) -> bool { true }
    fn check(&self, ctx: &FileContext) -> Vec<Diagnostic>;
    fn fix(&self, ctx: &FileContext) -> Option<String> { None }
}
```

## Config File (sqrust.toml)

```toml
[rules]
# Disable specific rules
"Layout/TrailingComma" = { enabled = false }

# Dialect selection
dialect = "ansi"  # ansi | bigquery | snowflake | redshift | duckdb | postgres | mysql

# Paths
include = ["models/**/*.sql", "*.sql"]
exclude = ["dbt_packages/**"]
```

## Dialect Priority (MVP)

1. ANSI SQL (baseline)
2. BigQuery (largest dbt user base)
3. Snowflake
4. DuckDB (growing fast, used for local testing)
5. PostgreSQL
6. Redshift
7. MySQL

## Testing

- Every rule has a test file in `sqrust-rules/tests/<rule_name>_test.rs`
- Test fixtures in `sqrust-rules/tests/fixtures/<category>/<rule>/`
  - `valid.sql` — SQL that should produce 0 violations
  - `invalid.sql` — SQL that should produce N violations
- TDD: write failing tests first, then implement
- Run tests: `cargo test`
- Run a single rule's tests: `cargo test -p sqrust-rules <rule_name>`

## Agents Available

| Agent | When to use |
|---|---|
| `workflow-orchestrator` | Start of any feature — get the execution plan |
| `codebase-reader` | Before planning — map relevant files |
| `web-searcher` | Need current docs, issues, changelog |
| `researcher` | Synthesise web-searcher data into decisions |
| `dialect-expert` | Adding/debugging dialect-specific syntax |
| `sql-parser-expert` | Parser design, AST node questions |
| `planner-analyser` | Feature design, architectural decisions |
| `coder` | Implement a specific, scoped part of a plan |
| `qa-engineer` | Write RED tests before implementation; GREEN verify after |
| `code-reviewer` | Review after implementation |
| `security-reviewer` | If feature touches external input or file paths |
| `doc-writer` | After feature complete, or to capture learnings |

## Performance Targets

- 1000 SQL files linted in < 2 seconds on a 4-core machine
- Zero allocations in the hot path (re-use AST walker)
- Startup time < 50ms (no JVM, no Python interpreter)

## What We Are NOT Building (v0.1)

- A full SQL execution engine
- A query planner or optimizer
- A database proxy or wire protocol server
- Anything requiring a network connection at lint time

## Documentation

```
docs/           Public — architecture, rule catalog, config reference
internals/      GITIGNORED — competition research, plans, decisions, learnings
  competition/  Per-tool competitive analysis
  plans/        YYYY-MM-DD-feature-name.md
  decisions/    YYYY-MM-DD-decision.md
  learnings/    YYYY-MM-DD-topic.md
```
