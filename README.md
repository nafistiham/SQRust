# SQRust

[![CI](https://github.com/nafistiham/SQRust/actions/workflows/ci.yml/badge.svg)](https://github.com/nafistiham/SQRust/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**A fast SQL linter written in Rust.** The Ruff for SQL.

300 rules. Single binary. No Python required.

---

## Why SQRust?

If you use sqlfluff, you know the pain: linting a 200-file dbt project takes minutes in CI. SQRust solves that.

Benchmarked on **495 SQL files** (jaffle-shop + attribution-playbook + mrr-playbook, combined real dbt corpus):

| Tool | Time | Rules |
|------|------|-------|
| **SQRust** | **42 ms** | **300** |
| sqruff | 79 ms | ~62 |
| sqlfluff 4.0.4 | 10,925 ms | ~89 |

> **2× faster than sqruff. 260× faster than sqlfluff. More rules than both combined.**

Measured with [hyperfine](https://github.com/sharkdp/hyperfine) (5+ runs, real corpus, all tools in ANSI mode).
[Run the benchmark yourself](#run-the-benchmark-yourself).

### Selective mode: top 50 rules

Don't need all 300 rules? Use `sqrust rules --disable` to trim to your essentials. Running just the top 50 most-used rules cuts the time in half:

| Config | Time | vs sqruff | vs sqlfluff |
|--------|------|-----------|-------------|
| SQRust — all 300 rules | **42 ms** | 1.9× faster | 260× faster |
| SQRust — top 50 rules | **21 ms** | 3.8× faster | 520× faster |
| sqruff | 79 ms | baseline | — |
| sqlfluff 4.0.4 | 10,925 ms | — | baseline |

```bash
# See all rules and their current status
sqrust rules

# Disable rules you don't need
sqrust rules --disable Convention/SelectStar

# Filter by category
sqrust rules --category Layout
```

---

## Install

**cargo (recommended):**

```bash
cargo install sqrust-cli
```

**Pre-built binary:**

```bash
# macOS (Apple Silicon)
curl -sSL https://github.com/nafistiham/SQRust/releases/latest/download/sqrust-aarch64-apple-darwin.tar.gz | tar -xz
sudo mv sqrust /usr/local/bin/

# macOS (Intel)
curl -sSL https://github.com/nafistiham/SQRust/releases/latest/download/sqrust-x86_64-apple-darwin.tar.gz | tar -xz
sudo mv sqrust /usr/local/bin/

# Linux (x86_64)
curl -sSL https://github.com/nafistiham/SQRust/releases/latest/download/sqrust-x86_64-unknown-linux-gnu.tar.gz | tar -xz
sudo mv sqrust /usr/local/bin/
```

**Build from source:**

```bash
git clone https://github.com/nafistiham/SQRust.git
cd SQRust
cargo build -p sqrust-cli --release
# binary at: target/release/sqrust
```

---

## Usage

```bash
# Lint a directory
sqrust check models/

# Lint a single file
sqrust check query.sql

# Auto-fix layout issues
sqrust fmt models/

# List all rules with enabled/disabled status
sqrust rules

# Enable or disable a specific rule
sqrust rules --disable Convention/SelectStar
sqrust rules --enable Convention/SelectStar
```

**Output:**

```
models/orders.sql:12:5: [Convention/SelectStar] Avoid SELECT *; list columns explicitly
models/orders.sql:34:1: [Layout/TrailingWhitespace] Trailing whitespace on line
models/payments.sql:8:24: [Convention/ColonCast] Avoid PostgreSQL-style ::cast; use CAST() instead
models/payments.sql:41:1: [Layout/LongLines] Line exceeds 120 characters (was 143)
```

---

## Rules

300 rules across 6 categories, mapped to sqlfluff's catalog where applicable.

| Category | Rules | Examples |
|----------|-------|---------|
| **Convention** | 63 | `SelectStar`, `ColonCast`, `NotEqual`, `IsNull`, `NoIsnullFunction`, `NoDualTable`, `NoNvl2` |
| **Layout** | 60 | `LongLines`, `TrailingWhitespace`, `ClauseOnNewLine`, `GroupByOnNewLine`, `OrderByOnNewLine` |
| **Lint** | 58 | `UnreferencedCTE`, `DuplicateCteNames`, `UnusedCte`, `SelectWithoutFrom`, `AddColumnWithoutDefault` |
| **Structure** | 57 | `WildcardInUnion`, `NaturalJoin`, `MaxSelectColumns`, `LateralJoin`, `WindowFrameFullPartition` |
| **Ambiguous** | 58 | `FloatingPointComparison`, `CastWithoutLength`, `UnsafeDivision`, `ConvertFunction` |
| **Capitalisation** | 4 | `KeywordCase`, `IdentifierCase` |

Full rule list → [docs/rules.md](docs/rules.md)

---

## Comparison

|  | SQRust | sqruff | sqlfluff |
|--|--------|--------|----------|
| Language | Rust | Rust | Python |
| Rules | **300** | ~62 | ~89 |
| Speed (495 files) | **42 ms** | 79 ms | 10,925 ms |
| Single binary | ✅ | ✅ | ❌ |
| Auto-fix | Partial (layout) | ✅ | ✅ |
| Config file | ✅ | ✅ | ✅ |
| Rule browser CLI | ✅ | ❌ | ❌ |
| Dialect support | ANSI | ANSI+ | Many |
| dbt-ready | ✅ | ✅ | ✅ |

---

## Configuration

Create a `sqrust.toml` in your project root. sqrust automatically finds it by walking up from the path you lint — no flags needed:

```toml
[sqrust]
exclude = ["dbt_packages/**", "target/**"]

[rules]
disable = [
    "Convention/SelectStar",
    "Layout/LongLines",
]
```

All 300 rules are enabled by default. Use `disable` to turn off specific rules by name, or use `sqrust rules --disable <rule>` to have it written automatically.

See [`sqrust.toml.example`](sqrust.toml.example) for a fully annotated template.

> **v0.2.0 (planned):** Ruff-style allowlist — opt into rule categories with `select = ["Convention"]` instead of opting out. Fields are reserved; using them now gives a clear error.

---

## Pre-commit

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/nafistiham/SQRust
    rev: v0.1.0
    hooks:
      - id: sqrust
        args: [check]
```

---

## Run the Benchmark Yourself

```bash
curl -sSL https://raw.githubusercontent.com/nafistiham/SQRust/main/bench/benchmark.sh | bash
```

Or clone and run:

```bash
git clone https://github.com/nafistiham/SQRust
cd SQRust/bench
./benchmark.sh            # uses jaffle-shop as corpus
./benchmark.sh /your/dbt/project   # or point at your own project
```

Requires: `hyperfine`, `sqlfluff`, `sqruff`, `sqrust` on PATH.

---

## Contributing

```bash
git clone https://github.com/nafistiham/SQRust
cd SQRust
cargo test --workspace   # all tests must pass
```

Each rule lives in `sqrust-rules/src/<category>/<rule_name>.rs` with a test file in `sqrust-rules/tests/`.
Every rule has ≥13 tests. We use TDD — tests first.

---

## License

MIT
