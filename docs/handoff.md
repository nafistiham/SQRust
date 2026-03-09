# SQRust — Handoff Document

> Last updated: 2026-03-09

---

## What It Is

**SQRust** is a SQL linter and formatter in Rust — a direct sqlfluff competitor targeting the dbt/data engineering market. The "Ruff for SQL" play.

- **Repo:** https://github.com/nafistiham/SQRust (private)
- **Target market:** dbt community — 50k teams, 100k members

---

## Competitive Landscape

| Tool | Rules | Dialects | Lang | Speed |
|------|-------|----------|------|-------|
| sqlfluff | ~80 | 20+ | Python | 8–15 min CI |
| sqruff | ~30 | 5 | Rust | 10x faster |
| sqlfmt | format only | dbt | Python | — |
| **SQRust** | **145** | ANSI→BigQuery→Snowflake→DuckDB→PG | **Rust** | **target: 1000 files < 2s** |

Our gap: sqlfluff's rule coverage + sqruff's speed + better dialect support.

---

## Current Status

**145 rules across 15 waves — all committed and pushed to main/develop.**

| Waves | Rules | Status |
|-------|-------|--------|
| 1–12 | 115 rules | ✅ Done |
| 13–15 | 30 more rules | ✅ Done |
| Wave 16 | Next 10 rules | ❌ Not started |

---

## Architecture

```
sqrust-core/    ← Rule trait, Diagnostic, FileContext, walker, types
sqrust-rules/   ← all rules (one file per rule, e.g. src/layout/trailing_comma.rs)
sqrust-cli/     ← main.rs, CLI (check / fmt / explain), clap
```

**Parser:** `sqlparser-rs` (NOT porting sqlfluff grammar)
**Parallelism:** `rayon` file-level
**Config:** `sqrust.toml` (TOML)
**CLI subcommands:** `check`, `fmt`, `explain`

---

## Rule Naming Convention

```
Category/RuleName
  Layout/TrailingComma
  Style/KeywordCase
  Lint/UnreferencedCTE
```

Each rule: `sqrust-rules/src/<category>/<rule_name>.rs`

---

## Config File (sqrust.toml)

```toml
[rules]
"Layout/TrailingComma" = { enabled = false }
dialect = "ansi"
include = ["models/**/*.sql", "*.sql"]
exclude = ["dbt_packages/**"]
```

---

## Testing

- Every rule has `sqrust-rules/tests/<rule_name>_test.rs`
- Fixtures: `sqrust-rules/tests/fixtures/<category>/<rule>/valid.sql` + `invalid.sql`
- `cargo test`
- Single rule: `cargo test -p sqrust-rules <rule_name>`

---

## Dependencies

```toml
sqlparser = "0.53"
serde = "1"
toml = "0.8"
clap = "4"
rayon = "1"
walkdir = "2"
```

---

## Key Agents

`dialect-expert`, `sql-parser-expert` (beyond standard set)
`internals/` is gitignored — competition research, plans, decisions

---

## Setup on New Machine

```bash
cd SQRust
cargo build --release
cargo test
```

---

## What To Do Next

1. **Plan Wave 16** — next 10 rules
2. **Dialect expansion** — BigQuery-specific rules (priority 2 after ANSI)
3. **Benchmarking** — compare against sqlfluff on real dbt projects
4. **CLI polish** — `explain` subcommand for rule documentation
