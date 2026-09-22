# SQRust — Product Overview

## What it is

SQRust is a SQL linter written in Rust. It is fast, comprehensive, and ships as a single binary with no runtime dependencies.

**The problem it solves:** sqlfluff is the default SQL linter for dbt projects, but it is slow — minutes to lint a large project in CI (47 seconds for 500 files on an Apple M-series machine; slower on typical CI runners). Teams disable it locally because waiting that long to commit is impractical. Violations only get caught in CI, hours after the code was written.

SQRust is the Ruff-for-SQL play: take the same idea that made Ruff successful for Python linting (compile everything, do one parallel pass), apply it to SQL.

---

## Benchmarks

Benchmark numbers live in a single place — the [README](../README.md#benchmarks) — so
they cannot drift out of sync. They are measured with
[hyperfine](https://github.com/sharkdp/hyperfine) over two public corpora, with all
tools in ANSI mode, and are reproducible via `bench/benchmark.sh`.

Summary: SQRust is roughly **2× faster than sqruff** and **an order of magnitude
faster than sqlfluff**, while running more rules. Rule counts are not directly
comparable between tools — see the README for that caveat.

---

## Key features

**330 rules** across 6 categories: Convention, Layout, Lint, Structure, Ambiguous, Capitalisation. See [docs/rules.md](rules.md) for the full catalog.

**Single binary.** No Python, no pip, no virtualenv. Install with `cargo install sqrust-cli` or download a pre-built binary.

**`sqrust rules` CLI.** Browse all 330 rules with their enabled/disabled status. Toggle rules without editing config manually.

```bash
sqrust rules                            # list all 330
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
    rev: v0.1.4
    hooks:
      - id: sqrust
        args: [check]
```

---

## Who it's for

**dbt teams running sqlfluff in CI.** If linting is slow enough that your team skips it locally, SQRust is the fix. Same rule concepts, an order of magnitude faster, single binary.

**Data engineering teams using pre-commit hooks.** No Python environment to manage in Docker or on developer machines.

**SQL-heavy projects with strict style requirements.** 330 rules covering style, correctness, and portability.

---

## Current scope and limitations

- **Auto-fix is partial.** `sqrust fmt` fixes 16 of the 330 rules — whitespace, spacing, blank lines, line endings, and a few Convention rewrites. The rest are report-only. Use `sqrust fmt --check` in CI to fail on unformatted files without writing.
- **dbt Jinja is not rendered.** Files containing `{{ ref(...) }}` fail to parse, so AST-based rules are skipped for them and only text-scanning rules apply. Run SQRust against compiled SQL (`target/compiled/`) for full coverage.
- **Rule thresholds are not yet configurable.** Rules with limits (line length, max joins, ...) use fixed defaults; `sqrust.toml` can disable a rule but not retune it. Per-rule settings are planned for v0.2.0.

---

## Roadmap

| Priority | Feature |
|----------|---------|
| Next | Per-rule configuration (thresholds in `sqrust.toml`) |
| v0.2.0 | Ruff-style `select` allowlist (opt-in rule selection) |
| Later | dbt Jinja-aware parsing |
| Later | Language Server Protocol support |
| ✅ Done | Dialect support (`--dialect`: BigQuery, Snowflake, DuckDB, Postgres, MySQL, ANSI) |
| ✅ Done | VS Code extension |
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
