# SQRust — Product Overview

## What it is

SQRust is a SQL linter written in Rust. It is fast, comprehensive, and ships as a single binary with no runtime dependencies.

**The problem it solves:** sqlfluff is the default SQL linter for dbt projects, but it is slow — 8+ minutes to lint a 200-file project in CI. Teams disable it locally because waiting that long to commit is impractical. Violations only get caught in CI, hours after the code was written.

SQRust is the Ruff-for-SQL play: take the same idea that made Ruff successful for Python linting (compile everything, do one parallel pass), apply it to SQL.

---

## Benchmarks

On 495 real SQL files from public dbt projects (jaffle-shop, attribution-playbook, mrr-playbook):

| Tool | Time | Rules |
|------|------|-------|
| **SQRust** | **42 ms** | **300** |
| sqruff | 79 ms | ~62 |
| sqlfluff | 10,925 ms | ~89 |

260× faster than sqlfluff. Rule counts are not directly comparable — see README for context.

**Selective mode** (top 50 rules only, using `sqrust rules --disable`):

| Tool | Time | Rules |
|------|------|-------|
| **SQRust (top 50)** | **~21 ms** | **50** |
| sqruff | 79 ms | ~62 |
| sqlfluff | 10,925 ms | ~89 |

---

## Key features

**300 rules** across 6 categories: Convention, Layout, Lint, Structure, Ambiguous, Capitalisation. See [docs/rules.md](rules.md) for the full catalog.

**Single binary.** No Python, no pip, no virtualenv. Install with `cargo install sqrust-cli` or download a pre-built binary.

**`sqrust rules` CLI.** Browse all 300 rules with their enabled/disabled status. Toggle rules without editing config manually.

```bash
sqrust rules                            # list all 300
sqrust rules --category Convention      # filter by category
sqrust rules --disable Layout/LongLines # write to sqrust.toml
sqrust rules --enable Layout/LongLines  # re-enable
```

**`sqrust.toml` config.** Auto-discovered by walking up from the linted path.

```toml
[sqrust]
exclude = ["dbt_packages/**", "target/**"]

[rules]
disable = ["Convention/SelectStar"]
```

**Pre-commit friendly.** Three lines of YAML, runs in under 100ms locally.

```yaml
repos:
  - repo: https://github.com/nafistiham/SQRust
    rev: v0.1.1
    hooks:
      - id: sqrust
        args: [check]
```

---

## Who it's for

**dbt teams running sqlfluff in CI.** If linting is slow enough that your team skips it locally, SQRust is the fix. Same rule concepts, 260× faster, single binary.

**Data engineering teams using pre-commit hooks.** No Python environment to manage in Docker or on developer machines.

**SQL-heavy projects with strict style requirements.** 300 rules covering style, correctness, and portability.

---

## Current scope and limitations

- **ANSI SQL only.** The parser (sqlparser-rs) supports multiple dialects, but SQRust currently uses ANSI mode. BigQuery support is next on the roadmap.
- **Auto-fix is partial.** `sqrust fmt` fixes layout violations (whitespace, indentation). Semantic rules (Convention, Lint, etc.) are report-only.
- **No VS Code extension yet.** CLI only.

---

## Roadmap

| Priority | Feature |
|----------|---------|
| Next | BigQuery dialect support |
| v0.2.0 | Ruff-style `select` allowlist (opt-in rule selection) |
| Later | Snowflake, DuckDB dialect support |
| Later | VS Code / Language Server Protocol extension |
| ✅ Done | Homebrew tap (`brew install nafistiham/tap/sqrust`) |

---

## Installation

```bash
# Homebrew (macOS)
brew install nafistiham/tap/sqrust

# Via cargo
cargo install sqrust-cli

# Pre-built binaries (no Rust required)
# macOS arm64, macOS x86_64, Linux x86_64, Windows x86_64
# https://github.com/nafistiham/SQRust/releases
```

---

## License

MIT. Open source.
